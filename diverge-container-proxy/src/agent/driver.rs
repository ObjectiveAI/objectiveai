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

use super::{Cmd, DequeueReply, Fate, Outcome, Queued, run};
use crate::encode::encoded;
use crate::proxy::Proxy;

/// A message offered to the loop in flight, and not yet fated.
struct Inflight {
    message: Queued,
    /// The server withdrew this message's key while this was on the
    /// wire: a fate the agent's server does not decide is `Dequeued`,
    /// and the message is not requeued.
    withdrawn: bool,
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
/// - Only the driver reads or writes any of this.
/// - A message is in exactly one place: the queue, the in-flight
///   slot, or a starting run's batch.
/// - The fate of a message on the wire is whatever that call to the
///   agent's server answers.
/// - The driver never awaits the agent's server: every call is a
///   task of its own, reporting back on a channel, so a dequeue
///   never waits on an `/enqueue` the loop holds open.
/// - The gate is the driver's: a run is launched, or a message
///   delivered, only while `fed`. The gate opens in the step that
///   reads a feed's success — the same step that answers the
///   server's `Fed` — and closes in the step that reads the loop's
///   end — the same step that writes `Inactive` — so the server never
///   hears `Fed` for a gate that then closes behind it.
/// - What the driver writes on the begin scope's main stream is
///   `Inactive`, with the rotating secrets collected, and — when the
///   collect failed — the one `Error` that ends the run, after which
///   the begin loop finishes the scope. The `Active` and the chunks
///   are [`run::relay`]'s.
struct Driver {
    queue: VecDeque<Queued>,
    inflight: Option<Inflight>,
    run: Run,
    /// The loop in flight refused a delivery, or missed one: nothing
    /// more is offered to it, and the next loop starts on the queue
    /// once it ends.
    halted: bool,
    /// Whether the program declared any secret: the gate is closed at
    /// the begin, and closed again by every loop's end, only then.
    gated: bool,
    /// The gate: whether the program holds secrets fed since the last
    /// loop ended — or needs none.
    fed: bool,
    /// Feeds waiting behind the one on the wire, in order.
    feeds: VecDeque<Feed>,
}

/// Start the driver for an agent container that has begun, and hand
/// back the channel the server's channels speak to it on. The
/// declaration is read here: it is set before the driver is.
pub fn driver(proxy: Arc<Proxy>, scope: Arc<ScopeHandle>) -> mpsc::UnboundedSender<Cmd> {
    let (sender, receiver) = mpsc::unbounded_channel();
    tokio::spawn(drive(proxy, scope, receiver));
    sender
}

/// The loop: one event, then the one post-step where runs are
/// launched, deliveries begun and feeds put on the wire.
///
/// The post-step runs after EVERY event, which is what closes the
/// race between a message and the loop's end: a loop ending before a
/// delivery's `missed` leaves the message in the slot until the
/// outcome requeues it, and the next post-step launches; a `missed`
/// before the loop's end halts, and the end's post-step launches the
/// drain. A loop's end closes the gate, when the program takes
/// secrets, and the post-step launches nothing until a feed opens it
/// again. The command channel closing is the connection gone — every
/// channel that held a sender is over — and the driver ends with it;
/// the calls in flight end with the process.
async fn drive(proxy: Arc<Proxy>, scope: Arc<ScopeHandle>, mut commands: mpsc::UnboundedReceiver<Cmd>) {
    let gated = proxy.gated();
    let mut driver = Driver {
        queue: VecDeque::new(),
        inflight: None,
        run: Run::Idle,
        halted: false,
        gated,
        fed: !gated,
        feeds: VecDeque::new(),
    };
    let mut starting: Option<oneshot::Receiver<Result<reqwest::Response, Error>>> = None;
    let mut ended: Option<oneshot::Receiver<Result<Values, Error>>> = None;
    let mut delivering: Option<oneshot::Receiver<Outcome>> = None;
    let mut feeding: Option<Feeding> = None;

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
                    let drained = withdrawn.len();
                    for message in withdrawn {
                        let _ = message.fate.send(Fate::Dequeued);
                    }
                    if let Some(inflight) = &mut driver.inflight {
                        if inflight.message.key == key {
                            inflight.withdrawn = true;
                        }
                    }
                    let _ = reply.send(DequeueReply {
                        drained,
                        active: matches!(driver.run, Run::Active),
                    });
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
                        ended = Some(run::relay(Arc::clone(&proxy), response, Arc::clone(&scope)));
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
                // The gate closes with the word, in one step: nothing
                // launches until the next feed opens it.
                driver.fed = !driver.gated;
                match collected {
                    Ok(Ok(values)) => send(&scope, &response::Frame::Inactive(values)).await,
                    Ok(Err(error)) => {
                        send(&scope, &response::Frame::Error(error)).await;
                        proxy.end();
                    }
                    Err(_) => {
                        send(&scope, &response::Frame::Error(uncollected())).await;
                        proxy.end();
                    }
                }
            },
            outcome = wait(delivering.as_mut()), if delivering.is_some() => {
                delivering = None;
                let Some(Inflight { message, withdrawn }) = driver.inflight.take() else {
                    continue;
                };
                match outcome.unwrap_or(Outcome::Failed) {
                    Outcome::Delivered => {
                        let _ = message.fate.send(Fate::Delivered);
                    }
                    Outcome::Dequeued => {
                        let _ = message.fate.send(Fate::Dequeued);
                    }
                    Outcome::Refused(error) => {
                        let _ = message.fate.send(Fate::Error(error));
                    }
                    Outcome::Missed | Outcome::Failed => {
                        if withdrawn {
                            let _ = message.fate.send(Fate::Dequeued);
                        } else {
                            driver.queue.push_front(message);
                            driver.halted = true;
                        }
                    }
                }
            },
            applied = wait(feeding.as_mut().map(|feeding| &mut feeding.outcome)), if feeding.is_some() => {
                let Some(Feeding { reply, .. }) = feeding.take() else {
                    continue;
                };
                // The gate opens with the word, in one step.
                match applied {
                    Ok(Ok(())) => {
                        driver.fed = true;
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

        // The post-step: the only place a run is launched, a
        // delivery begun, or a feed put on the wire.
        match driver.run {
            Run::Idle if driver.fed && !driver.queue.is_empty() => {
                let batch: Vec<Queued> = driver.queue.drain(..).collect();
                starting = Some(run::start(Arc::clone(&proxy), messages(&batch)));
                driver.run = Run::Starting(batch);
            }
            Run::Active if driver.fed && !driver.halted && driver.inflight.is_none() && !driver.queue.is_empty() => {
                let Some(message) = driver.queue.pop_front() else {
                    continue;
                };
                delivering = Some(super::deliver(Arc::clone(&proxy), message.key.clone(), message.content.clone()));
                driver.inflight = Some(Inflight {
                    message,
                    withdrawn: false,
                });
            }
            Run::Idle | Run::Starting(_) | Run::Active => {}
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
