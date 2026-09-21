//! The queue's driver: one task that owns the queue, the loop's
//! state, and every decision about what the agent's server is asked.

use std::collections::VecDeque;
use std::sync::Arc;

use diverge_provider_sdk::server::scope_handle::ScopeHandle;
use diverge_provider_sdk::shared::error::Error;
use rmcp::model::ContentBlock;
use tokio::sync::{mpsc, oneshot};

use super::{Cmd, DequeueReply, Fate, Outcome, Queued, run};
use crate::proxy::Proxy;
use crate::stamp::Stamp;

/// A message offered to the loop in flight, and not yet fated.
struct Inflight {
    message: Queued,
    /// The server withdrew the queue while this was on the wire: a
    /// fate the agent's server does not decide is `Dequeued`, and the
    /// message is not requeued.
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
/// - Nothing here writes an error on the begin scope's main stream.
struct Driver {
    queue: VecDeque<Queued>,
    inflight: Option<Inflight>,
    run: Run,
    /// The loop in flight refused a delivery, or missed one: nothing
    /// more is offered to it, and the next loop starts on the queue
    /// once it ends.
    halted: bool,
}

/// Start the driver for an agent container that has begun, and hand
/// back the channel the server's channels speak to it on.
pub fn driver(proxy: Arc<Proxy>, scope: Arc<ScopeHandle>, stamp: Stamp) -> mpsc::UnboundedSender<Cmd> {
    let (sender, receiver) = mpsc::unbounded_channel();
    tokio::spawn(drive(proxy, scope, stamp, receiver));
    sender
}

/// The loop: one event, then the one post-step where runs are
/// launched and deliveries begun.
///
/// The post-step runs after EVERY event, which is what closes the
/// race between a message and the loop's end: a loop ending before a
/// delivery's `missed` leaves the message in the slot until the
/// outcome requeues it, and the next post-step launches; a `missed`
/// before the loop's end halts, and the end's post-step launches the
/// drain. The command channel closing is the connection gone — every
/// channel that held a sender is over — and the driver ends with it;
/// the calls in flight end with the process.
async fn drive(proxy: Arc<Proxy>, scope: Arc<ScopeHandle>, stamp: Stamp, mut commands: mpsc::UnboundedReceiver<Cmd>) {
    let mut driver = Driver {
        queue: VecDeque::new(),
        inflight: None,
        run: Run::Idle,
        halted: false,
    };
    let mut starting: Option<oneshot::Receiver<Result<reqwest::Response, Error>>> = None;
    let mut ended: Option<oneshot::Receiver<()>> = None;
    let mut delivering: Option<oneshot::Receiver<Outcome>> = None;

    loop {
        tokio::select! {
            command = commands.recv() => match command {
                None => break,
                Some(Cmd::Enqueue(message)) => driver.queue.push_back(message),
                Some(Cmd::Dequeue(reply)) => {
                    let drained = driver.queue.len();
                    for message in driver.queue.drain(..) {
                        let _ = message.fate.send(Fate::Dequeued);
                    }
                    if let Some(inflight) = &mut driver.inflight {
                        inflight.withdrawn = true;
                    }
                    let _ = reply.send(DequeueReply {
                        drained,
                        active: matches!(driver.run, Run::Active),
                    });
                }
            },
            started = wait(&mut starting), if starting.is_some() => {
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
                        ended = Some(run::relay(response, Arc::clone(&scope), stamp.clone()));
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
            _ = wait(&mut ended), if ended.is_some() => {
                ended = None;
                driver.run = Run::Idle;
                driver.halted = false;
            },
            outcome = wait(&mut delivering), if delivering.is_some() => {
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
        }

        // The post-step: the only place a run is launched or a
        // delivery begun.
        match driver.run {
            Run::Idle if !driver.queue.is_empty() => {
                let batch: Vec<Queued> = driver.queue.drain(..).collect();
                let content = joined(&batch);
                starting = Some(run::start(Arc::clone(&proxy), content));
                driver.run = Run::Starting(batch);
            }
            Run::Active if !driver.halted && driver.inflight.is_none() && !driver.queue.is_empty() => {
                let Some(message) = driver.queue.pop_front() else {
                    continue;
                };
                delivering = Some(super::deliver(Arc::clone(&proxy), message.content.clone()));
                driver.inflight = Some(Inflight {
                    message,
                    withdrawn: false,
                });
            }
            Run::Idle | Run::Starting(_) | Run::Active => {}
        }
    }
}

/// The messages as one content: their blocks concatenated, in the
/// order they were enqueued — the join every harness sees for the
/// messages a loop finds waiting.
fn joined(batch: &[Queued]) -> Vec<ContentBlock> {
    batch.iter().flat_map(|message| message.content.iter().cloned()).collect()
}

/// The slot's answer, or forever when there is no slot: a branch
/// that is disabled is never polled, and this is what it would poll.
async fn wait<T>(slot: &mut Option<oneshot::Receiver<T>>) -> Result<T, oneshot::error::RecvError> {
    match slot {
        Some(receiver) => receiver.await,
        None => std::future::pending().await,
    }
}

/// The task that was to start the run ended without saying how.
fn died() -> Error {
    Error(serde_json::json!({
        "kind": "agent",
        "error": "the call that was to start the run died",
    }))
}
