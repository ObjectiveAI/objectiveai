//! The agent's conversation, off the run into its log; and the two
//! words that bracket each loop.

use std::sync::Arc;

use diverge_sdk::daemon::agent_log::{self as log, Item};
use diverge_sdk::provider::endpoints::containers::agents::run::client::execute::{Event, ExecuteStream};
use diverge_sdk::provider::endpoints::containers::agents::run::server::response::AgenticLoopChunk;
use diverge_sdk::provider::endpoints::containers::client::WaitError;
use futures_util::StreamExt as _;
use tokio::sync::oneshot;

use super::lifecycle::{self, End};
use super::mcp;
use super::{AgentRun, Decisions, Ended, activate, stop};
use crate::daemon::Daemon;
use crate::logs;
use crate::store::AgentId;

/// Read the run's stream to its end: every chunk kept, the agent
/// attested under its `_meta` — a user part as its sender's, by the
/// key the message was enqueued under — the loop's begin and end kept
/// as whether a loop runs (what that makes of the agent's activity is
/// [`activity`](super::activity)'s), the served tools started on the begin so
/// its calls find them up — each held up by its own use from then on,
/// and the dependencies run on for the agent's life — the container's
/// inactive, the delivered messages swept as rule 3 of
/// [`Message`](super::Message) states, and what its program made of its rotating secrets
/// written back and the run activated again while a message is
/// pending, as [`inactive_agent`](super::inactive_agent) states, a
/// failure kept as an `Error` item naming secrets and never a value,
/// and told to the agent's outbox pump, which feeds again for a batch
/// behind the gate —
/// and the stream's end or error the run over, told as the end it was
/// unless whoever ended the run said first: the connection gone the
/// provider lost, kept as no error; a finish the run exited; any other
/// failure kept as an `Error` item and the run failed. The end is
/// handed to a task of its own — see
/// [`end_agent`](stop::end_agent).
pub async fn pump(daemon: Arc<Daemon>, run: Arc<AgentRun>, mut stream: ExecuteStream) {
    let context = run.context(&daemon);
    let mut ended = Ended::Finished;
    while let Some(event) = stream.next().await {
        match event {
            Ok(Event::Chunk(mut chunk)) => {
                chunk.attest(&run.attestation);
                match chunk {
                    // A structured decision, or its answers: numbered,
                    // under the one lock, so the ids follow the order
                    // the items are written in.
                    AgenticLoopChunk::UserStructuredDecisions(chunk) => {
                        let (sender, echo) = echoed(&run, &chunk.decision.key).await;
                        let mut decisions = run.decisions.lock().await;
                        let id = numbering(&daemon, &run, &mut decisions).await.decision();
                        let item = Item::UserStructuredDecisions(log::UserStructuredDecisions { id, sender, chunk });
                        say(echo, kept(&daemon, &run, item).await);
                    }
                    AgenticLoopChunk::StructuredDecisionsResponse(chunk) => {
                        let mut decisions = run.decisions.lock().await;
                        let id = numbering(&daemon, &run, &mut decisions).await.response();
                        let item = Item::StructuredDecisionsResponse(log::StructuredDecisionsResponse { id, chunk });
                        let _ = append(&daemon, &run, item).await;
                    }
                    chunk => {
                        let (sender, echo) = match user_key(&chunk) {
                            Some(key) => echoed(&run, key).await,
                            None => (None, None),
                        };
                        let item = match sender {
                            Some(sender) => Item::User(log::User { sender, chunk }),
                            None => Item::Chunk(chunk),
                        };
                        say(echo, kept(&daemon, &run, item).await);
                    }
                }
            }
            Ok(Event::Active) => {
                run.loop_active.send_replace(true);
                run.touch();
                mcp::start_all(&context).await;
            }
            Ok(Event::Inactive(values)) => {
                run.loop_active.send_replace(false);
                run.touch();
                run.sweep_messages().await;
                if let Err(error) = activate::inactive_agent(&daemon, &run, values).await {
                    // The gate is left shut: a batch that crossed this
                    // inactive is behind it, and its sender feeds again.
                    run.unfed.notify_one();
                    let _ = append(
                        &daemon,
                        &run,
                        Item::Error(log::Error {
                            r#type: Default::default(),
                            error,
                        }),
                    )
                    .await;
                }
            }
            Err(WaitError::Closed) => {
                ended = Ended::Closed;
                break;
            }
            Err(error) => {
                let _ = append(
                    &daemon,
                    &run,
                    Item::Error(log::Error {
                        r#type: Default::default(),
                        error: diverge_sdk::shared::error::Error(serde_json::json!({
                            "kind": "run",
                            "error": error.to_string(),
                        })),
                    }),
                )
                .await;
                ended = Ended::Failed(error.to_string());
                break;
            }
        }
    }
    run.end_as(End::Unexpected(ended.reason())).await;
    stop::end_agent(daemon, run);
}

/// Where the numbering of the agent's structured decisions stands,
/// read back from its log the first time a run needs it: the highest
/// id any decision or response there has, and the latest decision's
/// where no response follows it. A log that cannot be read numbers
/// from nothing, which is `0` again: the items are kept either way.
async fn numbering<'a>(daemon: &Daemon, run: &AgentRun, decisions: &'a mut Option<Decisions>) -> &'a mut Decisions {
    if decisions.is_none() {
        let mut read = Decisions::default();
        let kept = logs::read(&daemon.logs, run.id, 1, u64::MAX).await.unwrap_or_default();
        for entry in kept {
            match entry.item {
                Item::UserStructuredDecisions(item) => {
                    read.highest = Some(read.highest.map_or(item.id, |highest| highest.max(item.id)));
                    read.open = Some(item.id);
                }
                Item::StructuredDecisionsResponse(item) => {
                    read.highest = Some(read.highest.map_or(item.id, |highest| highest.max(item.id)));
                    read.open = None;
                }
                _ => {}
            }
        }
        *decisions = Some(read);
    }
    decisions.get_or_insert_default()
}

/// Who sent the message enqueued under `key`, and — for the first item
/// kept under it — where its echo is said: rule 4 of
/// [`Message`](super::Message). Neither, for a key the run does not
/// hold.
async fn echoed(run: &AgentRun, key: &str) -> (Option<diverge_sdk::daemon::creator::Creator>, Option<oneshot::Sender<u64>>) {
    match run.messages.lock().await.get_mut(key) {
        Some(message) => (Some(message.sender.clone()), message.echo.take()),
        None => (None, None),
    }
}

/// The echo said, where this item is one and it was kept: its
/// `logs_index`. An item the log did not keep is no echo, and the
/// sender is dropped unsaid.
fn say(echo: Option<oneshot::Sender<u64>>, index: Option<u64>) {
    if let (Some(echo), Some(index)) = (echo, index) {
        let _ = echo.send(index);
    }
}

/// One item onto the run's log — the one it took at its start — under
/// its live lock, and the watchers told: its `logs_index`, or `None`
/// where the log kept nothing.
async fn kept(daemon: &Daemon, run: &AgentRun, item: Item) -> Option<u64> {
    lifecycle::note(daemon.logs.clone(), run.id, Arc::clone(&run.log), item).await
}

/// One item onto the run's log, as [`kept`] keeps it.
pub async fn append(daemon: &Daemon, run: &AgentRun, item: Item) -> Result<(), logs::Error> {
    let _ = kept(daemon, run, item).await;
    Ok(())
}

/// One item onto the log of the agent with the id, running or not; a
/// deleted agent's keeps nothing.
pub async fn append_for(daemon: &Daemon, id: AgentId, item: Item) -> Result<(), logs::Error> {
    let log = daemon.live.log(&daemon.store, id).await;
    let _ = lifecycle::note(daemon.logs.clone(), id, log, item).await;
    Ok(())
}

/// The key a user part carries: the message it landed from.
fn user_key(chunk: &AgenticLoopChunk) -> Option<&str> {
    match chunk {
        AgenticLoopChunk::UserTextContent(part) => Some(&part.key),
        AgenticLoopChunk::UserImageContent(part) => Some(&part.key),
        AgenticLoopChunk::UserAudioContent(part) => Some(&part.key),
        AgenticLoopChunk::UserResource(part) => Some(&part.key),
        AgenticLoopChunk::UserResourceLink(part) => Some(&part.key),
        _ => None,
    }
}
