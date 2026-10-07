//! The agent's conversation, off the run into its log; and the two
//! words that bracket each loop.

use std::sync::Arc;

use diverge_sdk::daemon::endpoints::agents::logs::server::response::{self as log, Item};
use diverge_sdk::provider::endpoints::containers::agents::run::client::execute::{Event, ExecuteStream};
use diverge_sdk::provider::endpoints::containers::agents::run::server::response::AgenticLoopChunk;
use futures_util::StreamExt as _;

use super::mcp::{self, Context};
use super::{AgentRun, Key, stop, tools};
use crate::daemon::Daemon;
use crate::logs;
use crate::store::AgentId;

/// Read the run's stream to its end: every chunk kept — a user part
/// as its sender's, by the key the message was enqueued under — the
/// loop's begin and end kept as whether the agent is active, the
/// served tools started on the begin and released on the end, and
/// the stream's end or error the run over.
pub async fn pump(daemon: Arc<Daemon>, run: Arc<AgentRun>, mut stream: ExecuteStream) {
    let context = Context {
        daemon: Arc::clone(&daemon),
        user: Key::Agent(run.id),
        root: run.name.clone(),
        chain: Vec::new(),
        served: Arc::clone(&run.served),
    };
    while let Some(event) = stream.next().await {
        match event {
            Ok(Event::Chunk(chunk)) => {
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
                run.touch();
                mcp::start_all(&context).await;
            }
            Ok(Event::Inactive) => {
                run.loop_active.send_replace(false);
                run.touch();
                let ids = run.served.lock().await.ids();
                for id in ids {
                    tools::release(&daemon, id, Key::Agent(run.id)).await;
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
