//! A message to an agent: the one way into it.

use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;

use diverge_sdk::daemon::creator::Creator;
use diverge_sdk::provider::endpoints::containers::client::UnaryError;
use diverge_sdk::provider::endpoints::containers::client::answered::Enqueue;
use diverge_sdk::shared::containers::enqueue;
use diverge_sdk::shared::error::Error;
use futures_util::future::BoxFuture;
use rmcp::model::ContentBlock;
use tokio::sync::Mutex;

use super::{AgentRun, Guard, Message, StartError, activate, events};
use crate::daemon::Daemon;
use crate::store::agents::Agent;

/// What became of a message.
pub enum Fate {
    /// The agent's program has it: the loop it started, or its own
    /// queue, taken at its next seam.
    Delivered,
    /// The container refused it, or the run is gone.
    Error(Error),
}

/// What a sent enqueue waits on: the proxy's word on the message, to
/// come, or why the enqueue never went out.
type Answer = Result<BoxFuture<'static, Result<enqueue::response::Frame, UnaryError<Enqueue>>>, UnaryError<Enqueue>>;

/// Send the message to the agent's run under a key the daemon mints,
/// with who sent it kept so its user parts are logged as theirs, and
/// hand back its fate to come: the proxy's word, once the agent's
/// program has the message — however long the run takes to be
/// activated and its loop to start. Nothing takes a message back once
/// it is in line. The rules, which make the messages that arrive
/// together one turn, in the order they arrived:
///
/// 1. **Counted, then in line.** The message is counted among the
///    agent's arriving — see [`Arrival`](crate::daemon::Arrival) — and
///    then asks the arrival lock, which serves it in arrival order.
/// 2. **Sent under the lock.** Holding it, the message gets the run —
///    started if it is not up, which every message behind it waits
///    out — is put on the graph, kept in the run's messages, counted
///    pending, and its enqueue SENT: the channel is out, its answer
///    not awaited. Then it is counted out; the one that counts the
///    arriving to zero is the last of its burst.
/// 3. **The last feeds.** After the lock, the last of a burst
///    activates the run — [`activate_agent`](super::activate_agent) —
///    which feeds the proxy only while its gate is shut; the feed is
///    the go, so the loop starts on every message of the burst,
///    joined in order. Every other leaves the feed to the one behind
///    it, which was counted before it was counted out. A message that
///    arrives after the feed is the next turn, or the proxy hands it
///    to the loop in flight.
/// 4. **An activation that fails** leaves the gate shut, and the
///    whole burst — the last message with it — waits behind it at the
///    proxy for the next feed: the message is sent, and nothing takes
///    it back, so its fate is the proxy's word whenever that comes.
/// 5. **In a task of its own**, waited for: a caller gone midway
///    neither breaks the order nor loses the feed its burst waits on.
///
/// The message is pending from its send until the future completes,
/// however it does: what an inactive asks before it feeds again. It
/// is on the graph from its send: one event from the sender to the
/// agent, whatever its fate, since the graph records what was sent
/// and not what landed. A run that could not be got is the error, and
/// nothing was sent.
pub async fn enqueue(daemon: &Arc<Daemon>, agent: &Agent, content: Vec<ContentBlock>, sender: Creator) -> Result<impl Future<Output = Fate>, StartError> {
    let key = uuid::Uuid::new_v4().to_string();
    let (daemon, agent) = (Arc::clone(daemon), agent.clone());
    let arrived = tokio::spawn(async move {
        let arrival = daemon.live.arriving(agent.id).await;
        let turn = arrival.lock.lock().await;
        let sent = match super::agent(&daemon, &agent).await {
            Ok(run) => Ok((send(&daemon, &run, key, content, sender).await, run)),
            Err(error) => Err(error),
        };
        let last = {
            let mut arriving = arrival.arriving.lock().await;
            *arriving -= 1;
            *arriving == 0
        };
        drop(turn);
        drop(arrival);
        daemon.live.arrived(agent.id).await;
        let (sent, run) = sent?;
        if last {
            // Rule 4: a failure leaves the burst behind the gate.
            let _ = activate::activate_agent(&daemon, &run).await;
        }
        Ok(sent)
    });
    let (pending, mut kept, answer) = arrived
        .await
        .unwrap_or_else(|error| Err(StartError::Unfinished(error.to_string())))?;
    Ok(async move {
        let _pending = pending;
        let fate = fated(answer).await;
        // Rules 2 and 3 of `Message`: an undelivered entry leaves now,
        // a delivered one is the pump's to sweep.
        let key = kept.key.take();
        if let Some(key) = key {
            let mut messages = kept.messages.lock().await;
            if matches!(fate, Fate::Delivered) {
                if let Some(message) = messages.get_mut(&key) {
                    message.delivered = true;
                }
            } else {
                messages.remove(&key);
            }
        }
        fate
    })
}

/// Rule 2's send, under the arrival lock: the message on the graph,
/// kept, counted pending — the count handed back, held until the fate
/// — and its enqueue out, the answer to come handed back with it.
async fn send(daemon: &Daemon, run: &Arc<AgentRun>, key: String, content: Vec<ContentBlock>, sender: Creator) -> (Guard, Kept, Answer) {
    events::record(daemon, &sender, &run.sender).await;
    run.messages.lock().await.insert(key.clone(), Message::new(sender));
    run.touch();
    let pending = run.pending.enter();
    let kept = Kept { messages: Arc::clone(&run.messages), key: Some(key.clone()) };
    let answer = run.handle.enqueue_sent(key, content).await;
    (pending, kept, answer)
}

/// The proxy's word on a sent enqueue, as the message's fate.
async fn fated(answer: Answer) -> Fate {
    let answered = match answer {
        Ok(answer) => answer.await,
        Err(error) => Err(error),
    };
    match answered {
        Ok(enqueue::response::Frame::Delivered) => Fate::Delivered,
        Ok(enqueue::response::Frame::Error(error)) => Fate::Error(error),
        Err(error) => Fate::Error(Error(serde_json::json!({
            "kind": "daemon",
            "error": format!("the message could not be enqueued: {error}"),
        }))),
    }
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
