//! The agent's conversation, off the run into its log; and the two
//! words that bracket each loop.

use std::sync::Arc;

use diverge_sdk::daemon::endpoints::agents::logs::server::response::{self as log, Item};
use diverge_sdk::provider::endpoints::containers::agents::run::client::execute::{Event, ExecuteStream};
use diverge_sdk::provider::endpoints::containers::agents::run::server::response::AgenticLoopChunk;
use diverge_sdk::shared::mcp::Who;
use futures_util::{StreamExt as _, future};

use super::mcp::{self, Context};
use super::{AgentRun, Caller, Key, User, stop, tools};
use crate::daemon::{Daemon, Kind};
use crate::logs;
use crate::store::AgentId;

/// Read the run's stream to its end: every chunk kept, the agent
/// attested under its `_meta` — a user part as its sender's, by the
/// key the message was enqueued under — the loop's begin and end kept
/// as whether a loop runs, the attached tools started on the begin
/// and released on the end — the dependencies run on, for the
/// agent's life — and the stream's end or error the run over.
pub async fn pump(daemon: Arc<Daemon>, run: Arc<AgentRun>, mut stream: ExecuteStream) {
    let context = Context {
        daemon: Arc::clone(&daemon),
        user: Key::Agent(run.id),
        caller: Caller::Agent {
            key: run.key.clone(),
            image: run.image.clone(),
        },
        served: Arc::clone(&run.served),
    };
    while let Some(event) = stream.next().await {
        match event {
            Ok(Event::Chunk(mut chunk)) => {
                chunk.attest(Some(&run.image), Who::Agent(&run.key));
                let item = match user_key(&chunk) {
                    Some(key) => match run.messages.lock().await.get(key).cloned() {
                        Some(sender) => Item::User(log::User { sender, chunk }),
                        None => Item::Chunk(chunk),
                    },
                    None => Item::Chunk(chunk),
                };
                let _ = append(&daemon, &run, item).await;
            }
            Ok(Event::Active) => {
                run.loop_active.send_replace(true);
                daemon.live.changed(Kind::Agents);
                run.touch();
                mcp::start_all(&context).await;
            }
            Ok(Event::Inactive) => {
                run.loop_active.send_replace(false);
                daemon.live.changed(Kind::Agents);
                run.touch();
                let attached = run.served.lock().await.attached_running();
                future::join_all(attached.iter().map(|key| tools::release(&daemon, *key, User::Container(Key::Agent(run.id))))).await;
                let mut served = run.served.lock().await;
                for key in attached {
                    served.idle(key);
                }
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
                break;
            }
        }
    }
    stop::ended_agent(&daemon, run).await;
}

/// One item onto the agent's log, under its live lock, and the
/// watchers told.
pub async fn append(daemon: &Daemon, run: &AgentRun, item: Item) -> Result<(), logs::Error> {
    append_for(daemon, run.id, item).await
}

/// One item onto the log of the agent with the id, running or not.
pub async fn append_for(daemon: &Daemon, id: AgentId, item: Item) -> Result<(), logs::Error> {
    let log = daemon.live.log(id).await;
    let _held = log.lock.lock().await;
    let wrapper = logs::append(&daemon.logs, id, item).await?;
    log.latest.send_replace(wrapper.logs_index);
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
