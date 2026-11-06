//! A message to an agent: the one way into it.

use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;

use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::shared::containers::enqueue;
use diverge_sdk::shared::error::Error;
use rmcp::model::ContentBlock;
use tokio::sync::Mutex;

use super::{AgentRun, Message, activate, events};
use crate::daemon::Daemon;

/// What became of a message.
pub enum Fate {
    /// The loop took it.
    Delivered,
    /// It was taken back before the loop took it.
    Cancelled,
    /// The container refused it, the run could not be activated for
    /// it, or the run is gone.
    Error(Error),
}

/// Enqueue the message under a key the daemon mints, with who sent
/// it kept so its user parts are logged as theirs, and wait for its
/// fate — however long the loop takes to reach a seam. The key is
/// what a cancel names. The message is pending — counted among the
/// agent's enqueues not yet fated — from before the run is activated
/// for it until its fate is known, so an inactive that crosses it
/// activates the run again; the activation failing is its fate.
pub async fn enqueue(daemon: &Daemon, run: &AgentRun, content: Vec<ContentBlock>, sender: Creator) -> Fate {
    let (_, fate) = enqueue_keyed(daemon, run, content, sender).await;
    fate.await
}

/// Enqueue the message, and hand back the key before the fate is
/// known, so that a cancel can name it: the fate is the future. The
/// message is pending from this call's return until the future
/// completes, however it does, and the run is activated for it when
/// the future is first polled — see [`activate_agent`](super::activate_agent)
/// — before the enqueue goes out; an activation that fails is the
/// fate, kind `secrets`. The message is on the graph from this call:
/// one event from the sender to the agent, whatever its
/// fate, since the graph records what was sent and not what landed.
pub async fn enqueue_keyed<'a>(daemon: &'a Daemon, run: &'a AgentRun, content: Vec<ContentBlock>, sender: Creator) -> (String, impl Future<Output = Fate> + 'a) {
    let key = uuid::Uuid::new_v4().to_string();
    events::record(daemon, &sender, &run.sender).await;
    run.messages.lock().await.insert(key.clone(), Message::new(sender));
    run.touch();
    let pending = run.pending.enter();
    let mut kept = Kept { messages: Arc::clone(&run.messages), key: Some(key.clone()) };
    let named = key.clone();
    let fate = async move {
        let _pending = pending;
        let fate = match activate::activate_agent(daemon, run).await {
            Ok(()) => match run.handle.enqueue(named.clone(), content).await {
                Ok(enqueue::response::Frame::Delivered) => Fate::Delivered,
                Ok(enqueue::response::Frame::Dequeued) => Fate::Cancelled,
                Ok(enqueue::response::Frame::Error(error)) => Fate::Error(error),
                Err(error) => Fate::Error(Error(serde_json::json!({
                    "kind": "daemon",
                    "error": format!("the message could not be enqueued: {error}"),
                }))),
            },
            Err(error) => Fate::Error(error),
        };
        // Rules 2 and 3 of `Message`: an undelivered entry leaves now,
        // a delivered one is the pump's to sweep.
        let key = kept.key.take();
        if let Some(key) = key {
            let mut messages = run.messages.lock().await;
            if matches!(fate, Fate::Delivered) {
                if let Some(message) = messages.get_mut(&key) {
                    message.delivered = true;
                }
            } else {
                messages.remove(&key);
            }
        }
        fate
    };
    (key, fate)
}

/// The entry of a message whose fate is not yet known: dropped still
/// holding its key — the fate's future dropped before it was known —
/// it removes the entry, as rule 2 of [`Message`] states. A drop cannot
/// wait on the lock, so the removal is a task of its own.
struct Kept {
    /// The run's messages.
    messages: Arc<Mutex<HashMap<String, Message>>>,
    /// The message's key, until its fate takes it.
    key: Option<String>,
}

impl Drop for Kept {
    fn drop(&mut self) {
        let Some(key) = self.key.take() else {
            return;
        };
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            let messages = Arc::clone(&self.messages);
            runtime.spawn(async move {
                messages.lock().await.remove(&key);
            });
        }
    }
}

/// Take the message under the key back, if the loop has not taken
/// it: its enqueue answers `Dequeued`.
pub async fn cancel(run: &AgentRun, key: &str) {
    let _ = run.handle.dequeue(key.to_string()).await;
}
