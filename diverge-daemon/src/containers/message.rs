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
use crate::daemon::{Arrival, Daemon};
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
/// it is sent. The rules, which make the messages that arrive
/// together one turn, in the order they arrived:
///
/// 1. **Stamped and counted as it is read.** The connection's read
///    loop gave the message its `arrival` — the daemon's next index,
///    and a count among the arriving to the agent reference it named —
///    before any lookup: see [`Live::arriving`](crate::daemon::Live::arriving).
///    The index is the order: the proxy orders the agent's messages
///    by it, and by nothing else.
/// 2. **Sent at once.** The message gets the run — started if it is
///    not up, two starts served one after the other — is put on the
///    graph, kept in the run's messages, counted pending, and its
///    enqueue SENT with its index: the channel is out, its answer not
///    awaited. It waits for no message before it.
/// 3. **Then counted out; the last feeds.** The one that counts the
///    arriving to zero is the last of its burst, and activates the run
///    — [`activate_agent`](super::activate_agent) — which feeds the
///    proxy only while its gate is shut; the feed is the go, so the
///    loop starts on every message of the burst, which the proxy joins
///    in index order. Every message counted before the last was sent
///    before it was counted out. Every other leaves the feed to the
///    last. A message that arrives after the feed is the next turn, or
///    the proxy hands it to the loop in flight.
/// 4. **One never sent counts out all the same** — a run that could
///    not be got, or a message refused before it came here: see
///    [`refused`] — and, the last of its burst, feeds the run another
///    of the burst was sent to, if any was.
/// 5. **An activation that fails** leaves the gate shut, and the
///    whole burst — the last message with it — waits behind it at the
///    proxy for the next feed: the message is sent, and nothing takes
///    it back, so its fate is the proxy's word whenever that comes.
/// 6. **In a task of its own**, waited for: a caller gone midway
///    loses neither the count nor the feed its burst waits on.
///
/// The message is pending from its send until the future completes,
/// however it does: what an inactive asks before it feeds again. It
/// is on the graph from its send: one event from the sender to the
/// agent, whatever its fate, since the graph records what was sent
/// and not what landed. A run that could not be got is the error, and
/// nothing was sent.
pub async fn enqueue(daemon: &Arc<Daemon>, arrival: Arrival, agent: &Agent, content: Vec<ContentBlock>, sender: Creator) -> Result<impl Future<Output = Fate>, StartError> {
    let key = uuid::Uuid::new_v4().to_string();
    let (daemon, agent) = (Arc::clone(daemon), agent.clone());
    let arrived = tokio::spawn(async move {
        let index = arrival.index;
        let sent = match super::agent(&daemon, &agent).await {
            Ok(run) => Ok((send(&daemon, &run, key, index, content, sender).await, run)),
            Err(error) => Err(error),
        };
        let run = sent.as_ref().ok().map(|(_, run)| Arc::clone(run));
        if let Some(run) = daemon.live.arrived(arrival, run).await {
            // Rule 5: a failure leaves the burst behind the gate.
            let _ = activate::activate_agent(&daemon, &run).await;
        }
        sent.map(|(sent, _)| sent)
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

/// A message that never came to [`enqueue`] — refused, or for an agent
/// there is none of — counted out all the same (rule 4): the last of
/// its burst feeds the run another of the burst was sent to, if any
/// was.
pub async fn refused(daemon: &Daemon, arrival: Arrival) {
    if let Some(run) = daemon.live.arrived(arrival, None).await {
        // Rule 5: a failure leaves the burst behind the gate.
        let _ = activate::activate_agent(daemon, &run).await;
    }
}

/// Rule 2's send: the message on the graph, kept, counted pending —
/// the count handed back, held until the fate — and its enqueue out
/// under its index, the answer to come handed back with it.
async fn send(daemon: &Daemon, run: &Arc<AgentRun>, key: String, index: u64, content: Vec<ContentBlock>, sender: Creator) -> (Guard, Kept, Answer) {
    events::record(daemon, &sender, &run.sender).await;
    run.messages.lock().await.insert(key.clone(), Message::new(sender));
    run.touch();
    let pending = run.pending.enter();
    let kept = Kept { messages: Arc::clone(&run.messages), key: Some(key.clone()) };
    let answer = run.handle.enqueue_sent(key, index, content).await;
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
