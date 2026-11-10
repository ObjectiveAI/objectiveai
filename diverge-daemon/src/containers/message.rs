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
    /// The loop took it.
    Delivered,
    /// It was taken back before the loop took it.
    Cancelled,
    /// The container refused it, the run could not be activated for
    /// it, or the run is gone.
    Error(Error),
}

/// What a sent enqueue waits on: the proxy's word on the message, to
/// come, or why the enqueue never went out.
type Answer = Result<BoxFuture<'static, Result<enqueue::response::Frame, UnaryError<Enqueue>>>, UnaryError<Enqueue>>;

/// Send the message to the agent's run under a key the daemon mints,
/// with who sent it kept so its user parts are logged as theirs, and
/// hand back the run and the key before the fate is known, so that a
/// cancel can name it: the fate is the future, however long the loop
/// takes to reach a seam — or `None`, the message withdrawn from its
/// `line` before it was sent. The rules, which make the messages that
/// arrive together one turn, in the order they arrived:
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
///    arrives after the feed is the next turn.
/// 4. **An activation that fails** withdraws the last message's own
///    enqueue, and is its fate, kind `secrets` — unless a loop took
///    it first. The burst before it waits behind the gate for the
///    next feed.
/// 5. **In a task of its own**, waited for: a caller cancelled midway
///    neither breaks the order nor loses the feed its burst waits on.
/// 6. **Withdrawn before its send** — [`Line::withdraw`] — the message
///    keeps its place and is counted out at its turn like any other,
///    but is never sent: no run got for it if it was withdrawn before
///    its turn, nothing on the graph, nothing kept or pending. Its
///    turn decides it, under the line's lock, so a withdrawal and the
///    send never both happen. The last of a burst withdrawn still
///    feeds, for the messages before it: the run that is up, while
///    any message is sent and not yet fated.
///
/// The message is pending from its send until the future completes,
/// however it does: what an inactive asks before it feeds again. It
/// is on the graph from its send: one event from the sender to the
/// agent, whatever its fate, since the graph records what was sent
/// and not what landed. A run that could not be got is the error, and
/// nothing was sent.
pub async fn enqueue(daemon: &Arc<Daemon>, agent: &Agent, content: Vec<ContentBlock>, sender: Creator, line: Line) -> Result<Option<(Arc<AgentRun>, String, impl Future<Output = Fate>)>, StartError> {
    let key = uuid::Uuid::new_v4().to_string();
    let (daemon, agent, named) = (Arc::clone(daemon), agent.clone(), key.clone());
    let arrived = tokio::spawn(async move {
        let arrival = daemon.live.arriving(agent.id).await;
        let turn = arrival.lock.lock().await;
        // Rule 6: withdrawn before its turn, it gets no run.
        let sent = if line.withdrawn().await {
            Ok(None)
        } else {
            match super::agent(&daemon, &agent).await {
                Ok(run) => Ok(send(&daemon, &run, &line, named, content, sender).await.map(|(pending, kept, answer)| (run, pending, kept, answer))),
                Err(error) => Err(error),
            }
        };
        let last = {
            let mut arriving = arrival.arriving.lock().await;
            *arriving -= 1;
            *arriving == 0
        };
        drop(turn);
        drop(arrival);
        daemon.live.arrived(agent.id).await;
        let Some((run, pending, kept, answer)) = sent? else {
            // Rule 6: the last withdrawn feeds the burst before it.
            if last
                && let Some(run) = daemon.live.agent_run(agent.id).await
                && run.pending.is_busy()
            {
                let _ = activate::activate_agent(&daemon, &run).await;
            }
            return Ok(None);
        };
        let activated = if last { activate::activate_agent(&daemon, &run).await } else { Ok(()) };
        Ok(Some((run, pending, kept, answer, activated)))
    });
    let Some((run, pending, mut kept, answer, activated)) = arrived
        .await
        .unwrap_or_else(|error| Err(StartError::Unfinished(error.to_string())))?
    else {
        return Ok(None);
    };
    let (taken, named) = (Arc::clone(&run), key.clone());
    let fate = async move {
        let _pending = pending;
        let fate = match activated {
            Ok(()) => fated(answer).await,
            // Rule 4.
            Err(error) => {
                cancel(&taken, &named).await;
                match fated(answer).await {
                    Fate::Cancelled => Fate::Error(error),
                    fate => fate,
                }
            }
        };
        // Rules 2 and 3 of `Message`: an undelivered entry leaves now,
        // a delivered one is the pump's to sweep.
        let key = kept.key.take();
        if let Some(key) = key {
            let mut messages = taken.messages.lock().await;
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
    Ok(Some((run, key, fate)))
}

/// Rule 2's send, under the arrival lock: the message on the graph,
/// kept, counted pending — the count handed back, held until the fate
/// — and its enqueue out, the answer to come handed back with it.
/// Rule 6 first, under the line's lock, held through the send: `None`,
/// nothing done, for a message withdrawn.
async fn send(daemon: &Daemon, run: &Arc<AgentRun>, line: &Line, key: String, content: Vec<ContentBlock>, sender: Creator) -> Option<(Guard, Kept, Answer)> {
    let mut place = line.0.lock().await;
    if *place == Place::Withdrawn {
        return None;
    }
    events::record(daemon, &sender, &run.sender).await;
    run.messages.lock().await.insert(key.clone(), Message::new(sender));
    run.touch();
    let pending = run.pending.enter();
    let kept = Kept { messages: Arc::clone(&run.messages), key: Some(key.clone()) };
    let answer = run.handle.enqueue_sent(key, content).await;
    *place = Place::Sent;
    Some((pending, kept, answer))
}

/// A message's place on its way to the run, shared by the way and by
/// the client's cancel: rule 6 of [`enqueue`]. Whichever decides it
/// first, under its lock, is what happened.
#[derive(Clone, Default)]
pub struct Line(Arc<Mutex<Place>>);

/// Where a message on its way stands.
#[derive(Default, PartialEq)]
enum Place {
    /// Not yet sent, nor withdrawn.
    #[default]
    Waiting,
    /// Its enqueue is out: a cancel is the provider's dequeue now.
    Sent,
    /// Taken out of line: it is never sent.
    Withdrawn,
}

impl Line {
    /// Take the message out of line, if it was not sent: `true` if it
    /// never will be, `false` if its enqueue is out already.
    pub async fn withdraw(&self) -> bool {
        let mut place = self.0.lock().await;
        if *place == Place::Sent {
            return false;
        }
        *place = Place::Withdrawn;
        true
    }

    /// Whether the message was taken out of line.
    async fn withdrawn(&self) -> bool {
        *self.0.lock().await == Place::Withdrawn
    }
}

/// The proxy's word on a sent enqueue, as the message's fate.
async fn fated(answer: Answer) -> Fate {
    let answered = match answer {
        Ok(answer) => answer.await,
        Err(error) => Err(error),
    };
    match answered {
        Ok(enqueue::response::Frame::Delivered) => Fate::Delivered,
        Ok(enqueue::response::Frame::Dequeued) => Fate::Cancelled,
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

/// Take the message under the key back, if the loop has not taken
/// it: its enqueue answers `Dequeued`.
pub async fn cancel(run: &AgentRun, key: &str) {
    let _ = run.handle.dequeue(key.to_string()).await;
}
