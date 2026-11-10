//! The queue's driver: one task that owns the queue, the loop's
//! state, the gate, and every decision about what the agent's server
//! is asked.

use std::collections::{HashSet, VecDeque};
use std::sync::Arc;

use diverge_sdk::container_proxy::outside::endpoints::agents::begin::server::response;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use diverge_sdk::shared::containers::secrets::Values;
use diverge_sdk::shared::error::Error;
use diverge_sdk::container_proxy::inside::agent::dequeue;
use diverge_sdk::container_proxy::inside::agent::run::request::Message;
use tokio::sync::{mpsc, oneshot};

use super::{Cmd, DequeueReply, Fate, Outcome, Queued, run};
use crate::encode::encoded;
use crate::proxy::Proxy;

/// The batch offered to the loop in flight — the agent's server's one
/// slot, filled — and not yet fated.
struct Offered {
    /// The messages on the wire, in enqueue order.
    batch: Vec<Queued>,
    /// The `/enqueue` that carries them: its outcome, to come.
    outcome: oneshot::Receiver<Outcome>,
    /// A `/dequeue` was sent for this batch: it is not sent twice.
    withdrawing: bool,
    /// The keys the server dequeued while this was on the wire: their
    /// messages are `Dequeued` unless the loop took the batch first,
    /// and are never offered again.
    cancelled: HashSet<String>,
    /// The server's dequeues of those keys, answered once the outcome
    /// is known.
    waiting: Vec<Waiting>,
}

/// A server's dequeue whose key the offered batch holds: its answer
/// waits for the batch's outcome, which the withdrawal makes prompt.
struct Waiting {
    /// Whether the queue gave back anything under the key already.
    drained: bool,
    /// Where the answer goes.
    reply: oneshot::Sender<DequeueReply>,
}

/// Whether a loop runs.
enum Run {
    /// No loop. The next message starts one.
    Idle,
    /// `/run` is on the wire, with these messages as its content; their
    /// fates follow its answer.
    Starting(Vec<Queued>),
    /// A loop runs, and its chunks are being relayed.
    Active,
}

/// A feed the server sent and the driver has not put on the wire.
struct Feed {
    values: Values,
    reply: oneshot::Sender<Result<(), Error>>,
}

/// The feed on the wire: the program's answer to come, and where the
/// server's channel hears it.
struct Feeding {
    outcome: oneshot::Receiver<Result<(), Error>>,
    reply: oneshot::Sender<Result<(), Error>>,
}

/// What the driver owns. Invariants, kept by there being one task:
///
/// 1. Only the driver reads or writes any of this.
/// 2. A message is in exactly one place: the queue, the offered
///    batch, or a starting run's batch.
/// 3. The queue is in enqueue order, always: a new message goes to
///    the back, and a batch that comes back goes to the FRONT, in its
///    own order, ahead of anything newer.
/// 4. Everything offered is offered whole: the `/run` takes the whole
///    queue, and so does the `/enqueue` — one batch, joined, which the
///    agent's server holds as its one message.
/// 5. At most one batch is offered, and none while a `/dequeue` is on
///    the wire: so a withdrawal can only ever land on the batch it was
///    sent for. A batch is withdrawn at most once.
/// 6. The fate of a batch on the wire is whatever that call to the
///    agent's server answers, and the messages in it share it — save
///    the ones the server dequeued while it was out, which a fate
///    other than `Delivered` answers `Dequeued` instead of offering
///    again.
/// 7. The driver never awaits the agent's server: every call is a
///    task of its own, reporting back on a channel, so a dequeue
///    never waits on an `/enqueue` the loop holds open — it waits on
///    the outcome, which the `/dequeue` sent for it hastens.
/// 8. The gate is the driver's: a run is launched, or a batch
///    offered, only while `fed`. It is shut at the begin, whatever
///    the program declared. It opens in the step that reads a feed's
///    success — the same step that answers the server's `Fed` — save
///    the deploy's feed, which leaves it shut; and it shuts in the
///    step that reads the loop's end — the same step that writes
///    `Inactive` — so the server never hears `Fed` for a gate that
///    then shuts behind it.
/// 9. What the driver writes on the begin scope's main stream is
///    `Inactive`, with the rotating secrets collected, and — when the
///    collect failed — the one `Error` that ends the run, after which
///    the begin loop finishes the scope. The `Active` and the chunks
///    are [`run::relay`]'s.
struct Driver {
    queue: VecDeque<Queued>,
    offered: Option<Offered>,
    run: Run,
    /// The loop in flight refused a batch, or missed one: nothing
    /// more is offered to it, and the next loop starts on the queue
    /// once it ends.
    halted: bool,
    /// The deploy's feed is still to come: the program registered
    /// something, so the provider feeds once before the run's id is
    /// out — before any message can be enqueued — and that feed
    /// applies the values and leaves the gate shut.
    registering: bool,
    /// The gate: whether a feed has come since the begin, or since the
    /// last loop ended. It is the go: the server feeds after the
    /// messages it means for one turn.
    fed: bool,
    /// Feeds waiting behind the one on the wire, in order.
    feeds: VecDeque<Feed>,
}

/// Start the driver for an agent container that has begun, and hand
/// back the channel the server's channels speak to it on.
/// `registering` is whether the program registered anything — a
/// dependency or a secret — which is when the provider feeds it at
/// the deploy.
pub fn driver(proxy: Arc<Proxy>, scope: Arc<ScopeHandle>, registering: bool) -> mpsc::UnboundedSender<Cmd> {
    let (sender, receiver) = mpsc::unbounded_channel();
    tokio::spawn(drive(proxy, scope, registering, receiver));
    sender
}

/// The loop: one event, then the one post-step where runs are
/// launched, batches offered, withdrawals sent and feeds put on the
/// wire.
///
/// The rules, by event:
///
/// 1. An enqueue goes to the back of the queue. While a batch is
///    offered and not yet being withdrawn, the post-step withdraws
///    it — `/dequeue`, once — so a burst of enqueues is one
///    withdrawal, and the batch that comes back is joined with them.
/// 2. A dequeue of a key answers every queued message under it
///    `Dequeued` at once. When the offered batch holds the key, the
///    key is recorded, the batch withdrawn, and the answer waits for
///    the batch's outcome; otherwise it is answered now.
/// 3. The batch's outcome: `Delivered` delivers every message in it —
///    a dequeue that came too late changes nothing. `Dequeued` answers
///    the recorded keys' messages `Dequeued` and puts the rest back at
///    the front, in order. `Missed` and `Failed` do the same, and halt
///    the loop. `Refused` answers every message with the error. Then
///    the waiting dequeues are answered.
/// 4. A withdrawal answered `dequeued` waits for the outcome it
///    hastens. One answered `empty` while its batch is still offered
///    may have overtaken that batch's `/enqueue` on the loopback — two
///    calls, two connections — or found the slot just taken: either
///    way it is sent again, until the outcome comes, which in both
///    cases is a loopback round trip away. One the agent's server
///    could not take answers the waiting dequeues with its error and
///    is not sent again for that batch; the keys stay recorded, so the
///    outcome still honours them when it comes.
/// 5. A feed goes on the wire behind any before it. Its success opens
///    the gate — every feed's, an empty one's too, save the deploy's
///    while `registering`, which only clears that — and the post-step
///    then launches `/run` on the whole queue, joined: the gate is
///    the go, and `/run` is the last call, after every enqueue that
///    reached the driver before it. A refusal leaves the gate as it
///    was.
///
/// The post-step runs after EVERY event, which is what closes the
/// race between a batch and the loop's end: a loop ending before a
/// batch's `missed` leaves the batch offered — and nothing launched —
/// until the outcome puts it back at the front, and the next
/// post-step launches; a `missed` before the loop's end halts, and
/// the end's post-step launches the drain. A loop's end shuts the
/// gate, and the post-step launches nothing until a feed opens it
/// again. The command channel closing
/// is the connection gone — every channel that held a sender is over
/// — and the driver ends with it; the calls in flight end with the
/// process.
async fn drive(proxy: Arc<Proxy>, scope: Arc<ScopeHandle>, registering: bool, mut commands: mpsc::UnboundedReceiver<Cmd>) {
    let mut driver = Driver {
        queue: VecDeque::new(),
        offered: None,
        run: Run::Idle,
        halted: false,
        registering,
        fed: false,
        feeds: VecDeque::new(),
    };
    let mut starting: Option<oneshot::Receiver<Result<reqwest::Response, Error>>> = None;
    let mut ended: Option<oneshot::Receiver<Result<Values, Error>>> = None;
    let mut feeding: Option<Feeding> = None;
    // The `/dequeue` on the wire: nothing is offered or launched until
    // it is answered, so it never lands on a batch it was not sent for.
    let mut withdrawal: Option<oneshot::Receiver<Result<dequeue::Outcome, Error>>> = None;
    // The last loop's `Inactive` or `Error`, on its way: the next loop's
    // relay waits for it, so the main stream keeps its order.
    let mut word: Option<tokio::task::JoinHandle<()>> = None;

    loop {
        tokio::select! {
            command = commands.recv() => match command {
                None => break,
                Some(Cmd::Enqueue(message)) => driver.queue.push_back(message),
                Some(Cmd::Dequeue { key, reply }) => {
                    // One pass, the messages under the key out and
                    // answered, every other kept in its order.
                    let (withdrawn, kept): (Vec<Queued>, Vec<Queued>) =
                        driver.queue.drain(..).partition(|message| message.key == key);
                    driver.queue.extend(kept);
                    let drained = !withdrawn.is_empty();
                    for message in withdrawn {
                        let _ = message.fate.send(Fate::Dequeued);
                    }
                    match &mut driver.offered {
                        Some(offered) if offered.batch.iter().any(|message| message.key == key) => {
                            offered.cancelled.insert(key);
                            offered.waiting.push(Waiting { drained, reply });
                        }
                        Some(_) | None => {
                            let _ = reply.send(if drained { DequeueReply::Dequeued } else { DequeueReply::Empty });
                        }
                    }
                }
                Some(Cmd::Feed { values, reply }) => driver.feeds.push_back(Feed { values, reply }),
            },
            started = wait(starting.as_mut()), if starting.is_some() => {
                starting = None;
                let batch = match std::mem::replace(&mut driver.run, Run::Idle) {
                    Run::Starting(batch) => batch,
                    Run::Idle | Run::Active => Vec::new(),
                };
                match started {
                    Ok(Ok(response)) => {
                        for message in batch {
                            let _ = message.fate.send(Fate::Delivered);
                        }
                        ended = Some(run::relay(Arc::clone(&proxy), response, Arc::clone(&scope), word.take()));
                        driver.run = Run::Active;
                    }
                    Ok(Err(error)) => {
                        for message in batch {
                            let _ = message.fate.send(Fate::Error(error.clone()));
                        }
                    }
                    Err(_) => {
                        for message in batch {
                            let _ = message.fate.send(Fate::Error(died()));
                        }
                    }
                }
            },
            collected = wait(ended.as_mut()), if ended.is_some() => {
                ended = None;
                driver.run = Run::Idle;
                driver.halted = false;
                // The gate shuts with the word, in one step: nothing
                // launches until the next feed opens it.
                driver.fed = false;
                // Sent on a task of its own: the main stream is paced
                // by the daemon's reading (see `credit`), and the
                // driver waiting on it would hold every feed, enqueue
                // and dequeue behind it. Order holds: the relay's last
                // frame went before `ended` fired, and the next loop's
                // relay waits for this one (`word`).
                let (frame, ends) = match collected {
                    Ok(Ok(values)) => (response::Frame::Inactive(values), false),
                    Ok(Err(error)) => (response::Frame::Error(error), true),
                    Err(_) => (response::Frame::Error(uncollected()), true),
                };
                let (scope, proxy) = (Arc::clone(&scope), Arc::clone(&proxy));
                word = Some(tokio::spawn(async move {
                    send(&scope, &frame).await;
                    if ends {
                        proxy.end();
                    }
                }));
            },
            outcome = wait(driver.offered.as_mut().map(|offered| &mut offered.outcome)), if driver.offered.is_some() => {
                let Some(Offered { batch, cancelled, waiting, .. }) = driver.offered.take() else {
                    continue;
                };
                // Whether the recorded keys' messages were withdrawn:
                // every fate but `Delivered` and `Refused` gives the
                // batch back, and they do not go back with it.
                let mut withdrawn = false;
                match outcome.unwrap_or(Outcome::Failed) {
                    Outcome::Delivered => {
                        for message in batch {
                            let _ = message.fate.send(Fate::Delivered);
                        }
                    }
                    Outcome::Refused(error) => {
                        for message in batch {
                            let _ = message.fate.send(Fate::Error(error.clone()));
                        }
                    }
                    given @ (Outcome::Dequeued | Outcome::Missed | Outcome::Failed) => {
                        if !matches!(given, Outcome::Dequeued) {
                            driver.halted = true;
                        }
                        let (dequeued, kept): (Vec<Queued>, Vec<Queued>) =
                            batch.into_iter().partition(|message| cancelled.contains(&message.key));
                        withdrawn = !dequeued.is_empty();
                        for message in dequeued {
                            let _ = message.fate.send(Fate::Dequeued);
                        }
                        // Back to the FRONT, in their own order, ahead
                        // of anything enqueued since.
                        for message in kept.into_iter().rev() {
                            driver.queue.push_front(message);
                        }
                    }
                }
                for Waiting { drained, reply } in waiting {
                    let _ = reply.send(if drained || withdrawn { DequeueReply::Dequeued } else { DequeueReply::Empty });
                }
            },
            withdrawn = wait(withdrawal.as_mut()), if withdrawal.is_some() => {
                withdrawal = None;
                // No batch: the outcome came first, and settled it —
                // and the post-step below offers what came back, so
                // nothing here may skip it.
                if let Some(offered) = &mut driver.offered {
                    match withdrawn {
                        Ok(Ok(dequeue::Outcome::Dequeued)) => {}
                        // Overtaken, or just taken: asked again by the
                        // post-step, until the outcome comes.
                        Ok(Ok(dequeue::Outcome::Empty)) => offered.withdrawing = false,
                        // Answered now, in the server's words: the
                        // outcome may be held for the loop's life
                        // without it.
                        Ok(Err(error)) => {
                            for Waiting { reply, .. } in offered.waiting.drain(..) {
                                let _ = reply.send(DequeueReply::Error(error.clone()));
                            }
                        }
                        Err(_) => {
                            for Waiting { reply, .. } in offered.waiting.drain(..) {
                                let _ = reply.send(DequeueReply::Error(unwithdrawn()));
                            }
                        }
                    }
                }
            },
            applied = wait(feeding.as_mut().map(|feeding| &mut feeding.outcome)), if feeding.is_some() => {
                let Some(Feeding { reply, .. }) = feeding.take() else {
                    continue;
                };
                // The gate opens with the word, in one step — save at
                // the deploy's feed (rule 5).
                match applied {
                    Ok(Ok(())) => {
                        if driver.registering {
                            driver.registering = false;
                        } else {
                            driver.fed = true;
                        }
                        let _ = reply.send(Ok(()));
                    }
                    Ok(Err(error)) => {
                        let _ = reply.send(Err(error));
                    }
                    Err(_) => {
                        let _ = reply.send(Err(unfed()));
                    }
                }
            },
        }

        // The post-step: the only place a run is launched, a batch
        // offered or withdrawn, or a feed put on the wire.
        if let Some(offered) = &mut driver.offered {
            // Something newer waits, or a key in it was dequeued: the
            // slot is asked back, and what comes back is joined.
            if !offered.withdrawing && withdrawal.is_none() && (!driver.queue.is_empty() || !offered.cancelled.is_empty()) {
                offered.withdrawing = true;
                withdrawal = Some(super::withdraw(Arc::clone(&proxy)));
            }
        } else if withdrawal.is_none() {
            match driver.run {
                Run::Idle if driver.fed && !driver.queue.is_empty() => {
                    let batch: Vec<Queued> = driver.queue.drain(..).collect();
                    starting = Some(run::start(Arc::clone(&proxy), messages(&batch)));
                    driver.run = Run::Starting(batch);
                }
                Run::Active if driver.fed && !driver.halted && !driver.queue.is_empty() => {
                    let batch: Vec<Queued> = driver.queue.drain(..).collect();
                    driver.offered = Some(Offered {
                        outcome: super::deliver(Arc::clone(&proxy), messages(&batch)),
                        batch,
                        withdrawing: false,
                        cancelled: HashSet::new(),
                        waiting: Vec::new(),
                    });
                }
                Run::Idle | Run::Starting(_) | Run::Active => {}
            }
        }
        if feeding.is_none() {
            if let Some(Feed { values, reply }) = driver.feeds.pop_front() {
                feeding = Some(Feeding {
                    outcome: run::feed(Arc::clone(&proxy), values),
                    reply,
                });
            }
        }
    }
}

/// The batch as the loop is asked it: every message, under its key,
/// in the order they were enqueued.
fn messages(batch: &[Queued]) -> Vec<Message> {
    batch
        .iter()
        .map(|message| Message {
            key: message.key.clone(),
            content: message.content.clone(),
        })
        .collect()
}

/// The slot's answer, or forever when there is no slot: a branch
/// that is disabled is never polled, and this is what it would poll.
async fn wait<T>(slot: Option<&mut oneshot::Receiver<T>>) -> Result<T, oneshot::error::RecvError> {
    match slot {
        Some(receiver) => receiver.await,
        None => std::future::pending().await,
    }
}

/// One frame on the begin scope's main stream, when it encoded.
async fn send(scope: &ScopeHandle, frame: &response::Frame) {
    if let Some(payload) = encoded(frame) {
        scope.send_response(&payload).await;
    }
}

/// The task that was to start the run ended without saying how.
fn died() -> Error {
    Error(serde_json::json!({
        "kind": "agent",
        "error": "the call that was to start the run died",
    }))
}

/// The task that was to withdraw the batch ended without saying how.
fn unwithdrawn() -> Error {
    Error(serde_json::json!({
        "kind": "agent",
        "error": "the call that was to withdraw the batch died",
    }))
}

/// The task that was to collect the secrets ended without saying how.
fn uncollected() -> Error {
    Error(serde_json::json!({
        "kind": "secrets",
        "error": "the task that was to collect the secrets died",
    }))
}

/// The task that was to feed the secrets ended without saying how.
fn unfed() -> Error {
    Error(serde_json::json!({
        "kind": "secrets",
        "error": "the task that was to feed the secrets died",
    }))
}
