//! The queue's driver: one task that owns the queue, the loop's
//! state, the gate, and every decision about what the agent's server
//! is asked.

use std::collections::VecDeque;
use std::sync::Arc;

use diverge_sdk::container_proxy::outside::endpoints::agents::begin::server::response;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use diverge_sdk::shared::containers::secrets::Values;
use diverge_sdk::shared::error::Error;
use diverge_sdk::container_proxy::inside::agent::run::request::Message;
use tokio::sync::{mpsc, oneshot};

use super::{Cmd, Fate, Outcome, Queued, run};
use crate::encode::encoded;
use crate::proxy::Proxy;

/// The batch handed to the loop in flight — one `/enqueue` on the
/// wire — and not yet answered.
struct Delivering {
    /// The messages on the wire, in enqueue order.
    batch: Vec<Queued>,
    /// The `/enqueue` that carries them: its outcome, to come.
    outcome: oneshot::Receiver<Outcome>,
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
/// 2. A message is in exactly one place until its fate is sent: the
///    queue, the batch being delivered, or a starting run's batch.
/// 3. The queue is in enqueue order, always: a new message goes to
///    the back, and a batch that comes back goes to the FRONT, in its
///    own order, ahead of anything newer.
/// 4. Everything sent is sent whole: the `/run` takes the whole
///    queue, and so does the `/enqueue` — one batch, joined, which the
///    agent's server takes as one user turn.
/// 5. At most one call carrying messages is on the wire — a `/run` or
///    an `/enqueue` — so the agent's server receives them in enqueue
///    order, and a batch that comes back is never overtaken by a
///    newer one.
/// 6. A message's fate is the answer to the call that carried it, and
///    the messages in it share it: `Delivered` on the `/run`'s `2xx`
///    or the `/enqueue`'s `204`, the error on a refusal. Nothing is
///    withdrawn, and no fate waits for a turn.
/// 7. The driver never awaits the agent's server: every call is a
///    task of its own, reporting back on a channel.
/// 8. The gate is the driver's: a run is launched, or a batch
///    delivered, only while `fed`. It is shut at the begin, whatever
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
    delivering: Option<Delivering>,
    run: Run,
    /// The loop in flight closed under a batch, or would not take one:
    /// nothing more is sent to it, and the next loop starts on the
    /// queue once it ends.
    closed: bool,
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
/// launched, batches delivered and feeds put on the wire.
///
/// The rules, by event:
///
/// 1. An enqueue goes to the back of the queue.
/// 2. The `/run`'s answer: a `2xx` delivers every message in its
///    batch and starts the relay; anything else answers every one
///    with the error.
/// 3. The `/enqueue`'s outcome: `Delivered` delivers every message in
///    the batch. `Returned` — the run closed under it, or the agent's
///    server could not take it — puts the batch back at the front, in
///    its order, and closes the loop in flight to further batches, so
///    the next `/run` starts on it. `Refused` answers every message
///    with the error.
/// 4. The loop's end makes the run idle and open to batches again,
///    and shuts the gate, in the step that writes `Inactive`.
/// 5. A feed goes on the wire behind any before it. Its success opens
///    the gate — every feed's, an empty one's too, save the deploy's
///    while `registering`, which only clears that — and the post-step
///    then launches `/run` on the whole queue, joined: the gate is
///    the go, and `/run` is the last call, after every enqueue that
///    reached the driver before it. A refusal leaves the gate as it
///    was.
///
/// The post-step runs after EVERY event. While an `/enqueue` is on
/// the wire it sends nothing; otherwise, behind an open gate and with
/// a queue, it launches `/run` with the whole of it when no loop
/// runs, and sends the whole of it as one `/enqueue` while one runs
/// and is not closed. That is what closes the race between a batch
/// and the loop's end: a loop ending before its batch's `409` leaves
/// the batch on the wire — and nothing launched — until the outcome
/// puts it back at the front, and the next post-step launches; a
/// `409` before the loop's end closes the loop, and the drain is
/// launched once the end has come and a feed has opened the gate.
/// The command channel closing is the connection gone — every
/// channel that held a sender is over — and the driver ends with it;
/// the calls in flight end with the process.
async fn drive(proxy: Arc<Proxy>, scope: Arc<ScopeHandle>, registering: bool, mut commands: mpsc::UnboundedReceiver<Cmd>) {
    let mut driver = Driver {
        queue: VecDeque::new(),
        delivering: None,
        run: Run::Idle,
        closed: false,
        registering,
        fed: false,
        feeds: VecDeque::new(),
    };
    let mut starting: Option<oneshot::Receiver<Result<reqwest::Response, Error>>> = None;
    let mut ended: Option<oneshot::Receiver<Result<Values, Error>>> = None;
    let mut feeding: Option<Feeding> = None;
    // The last loop's `Inactive` or `Error`, on its way: the next loop's
    // relay waits for it, so the main stream keeps its order.
    let mut word: Option<tokio::task::JoinHandle<()>> = None;

    loop {
        tokio::select! {
            command = commands.recv() => match command {
                None => break,
                Some(Cmd::Enqueue(message)) => driver.queue.push_back(message),
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
                driver.closed = false;
                // The gate shuts with the word, in one step: nothing
                // launches until the next feed opens it.
                driver.fed = false;
                // Sent on a task of its own: the main stream is paced
                // by the daemon's reading (see `credit`), and the
                // driver waiting on it would hold every feed and
                // enqueue behind it. Order holds: the relay's last
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
            outcome = wait(driver.delivering.as_mut().map(|delivering| &mut delivering.outcome)), if driver.delivering.is_some() => {
                let Some(Delivering { batch, .. }) = driver.delivering.take() else {
                    continue;
                };
                match outcome.unwrap_or(Outcome::Returned) {
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
                    Outcome::Returned => {
                        // Nothing more for this loop: the next starts
                        // on the batch.
                        driver.closed = true;
                        // Back to the FRONT, in their own order, ahead
                        // of anything enqueued since.
                        for message in batch.into_iter().rev() {
                            driver.queue.push_front(message);
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
        // delivered, or a feed put on the wire.
        if driver.delivering.is_none() && driver.fed && !driver.queue.is_empty() {
            match driver.run {
                Run::Idle => {
                    let batch: Vec<Queued> = driver.queue.drain(..).collect();
                    starting = Some(run::start(Arc::clone(&proxy), messages(&batch)));
                    driver.run = Run::Starting(batch);
                }
                Run::Active if !driver.closed => {
                    let batch: Vec<Queued> = driver.queue.drain(..).collect();
                    driver.delivering = Some(Delivering {
                        outcome: super::deliver(Arc::clone(&proxy), messages(&batch)),
                        batch,
                    });
                }
                Run::Starting(_) | Run::Active => {}
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
