//! The container's own queue, mirrored onto the proxy's.
//!
//! The proxy holds the queue that matters — it folds a pending
//! prompt onto the next tool response, at its head, the moment that
//! response goes to the agent — but it reports nothing to this
//! container except the RETURN of its `/enqueue` call. So every
//! message lives here too, with its fate, and the proxy's return is
//! the delivery signal: when `/enqueue` comes back `attached`, the
//! moment is recorded, and the runner yields the prompt as a `user`
//! chunk after the tool response whose completion most closely
//! follows it.
//!
//! # Three ways a fate is decided, race-free
//!
//! A message's fate sender sits behind a lock, and whoever takes it
//! first decides: the fold (`delivered`), the caller's dequeue
//! (`dequeued`), or the run's end (`delivered` — the message opens
//! the next turn as its prompt). A late proxy answer for a message
//! somebody else already decided does nothing.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use diverge_provider_sdk::agentic_loop_container::enqueue::Response;
use tokio::sync::oneshot;
use tokio::sync::Mutex;

use super::proxy;
use super::proxy::Enqueued;

/// The one queue, alive as long as the container.
pub static QUEUE: Queue = Queue {
    state: Mutex::const_new(State {
        pending: Vec::new(),
        delivered: Vec::new(),
        closed: false,
    }),
};

/// The queue: what is pending, what was folded and not yet spoken,
/// and whether the run has ended.
pub struct Queue {
    /// Everything, behind one lock never held across a proxy call.
    state: Mutex<State>,
}

/// The queue's state.
struct State {
    /// Messages whose fate is undecided.
    pending: Vec<Arc<Pending>>,
    /// Messages the proxy folded, waiting for the runner to yield
    /// them after the matching tool response.
    delivered: Vec<Delivery>,
    /// The run has ended: everything later is `missed`.
    closed: bool,
}

/// One message, and the one right to decide its fate.
pub struct Pending {
    /// The prompt, verbatim.
    prompt: String,
    /// The fate sender; taken by whoever decides first.
    fate: Mutex<Option<oneshot::Sender<Response>>>,
}

impl Pending {
    /// Decide the fate, if nobody has. Whether this call did.
    async fn settle(&self, fate: Response) -> bool {
        match self.fate.lock().await.take() {
            Some(sender) => {
                let _ = sender.send(fate);
                true
            }
            None => false,
        }
    }
}

/// A folded message, and when the fold happened.
struct Delivery {
    /// The prompt, verbatim.
    prompt: String,
    /// The proxy's return, as seconds since the epoch — the clock
    /// the gateway stamps its events with.
    at: f64,
}

/// Now, on the gateway's clock.
fn now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_secs_f64())
        .unwrap_or(0.0)
}

impl Queue {
    /// Queue a message: here, and with the proxy. The receiver
    /// resolves to the fate whenever it is decided; on a closed
    /// queue, `missed` at once.
    pub async fn enqueue(&self, prompt: String) -> oneshot::Receiver<Response> {
        let (sender, receiver) = oneshot::channel();
        let pending = {
            let mut state = self.state.lock().await;
            if state.closed {
                let _ = sender.send(Response::Missed {
                    r#type: Default::default(),
                });
                return receiver;
            }
            let pending = Arc::new(Pending {
                prompt: prompt.clone(),
                fate: Mutex::new(Some(sender)),
            });
            state.pending.push(pending.clone());
            pending
        };
        tokio::spawn(async move {
            match proxy::enqueue(prompt.clone()).await {
                Ok(Enqueued::Attached { .. }) => {
                    let at = now();
                    let delivered = pending
                        .settle(Response::Delivered {
                            r#type: Default::default(),
                        })
                        .await;
                    if delivered {
                        let mut state = QUEUE.state.lock().await;
                        state.pending.retain(|other| !Arc::ptr_eq(other, &pending));
                        state.delivered.push(Delivery { prompt, at });
                    }
                }
                // Whoever dequeued it decided its fate already.
                Ok(Enqueued::Dequeued { .. }) => {}
                // The proxy could not be asked: the message stays
                // pending here, and the run's end takes it as the
                // next turn's prompt.
                Err(_) => {}
            }
        });
        receiver
    }

    /// The caller's dequeue: every pending message answers
    /// `dequeued`, and the proxy is told to fold none of them.
    /// Whether there was anything to withdraw.
    pub async fn dequeue(&self) -> bool {
        let taken = std::mem::take(&mut self.state.lock().await.pending);
        if taken.is_empty() {
            return false;
        }
        let _ = proxy::dequeue().await;
        let mut any = false;
        for pending in taken {
            any |= pending
                .settle(Response::Dequeued {
                    r#type: Default::default(),
                })
                .await;
        }
        any
    }

    /// The run's last look: whatever is still pending becomes the
    /// next turn's prompt — the proxy told to fold none of it, each
    /// message answered `delivered` — and an empty queue is CLOSED
    /// in the same lock hold that proved it empty, so no message
    /// can land in the gap and wait forever.
    pub async fn take_or_close(&self) -> Vec<String> {
        let taken = {
            let mut state = self.state.lock().await;
            let taken = std::mem::take(&mut state.pending);
            if taken.is_empty() {
                state.closed = true;
            }
            taken
        };
        if taken.is_empty() {
            return Vec::new();
        }
        let _ = proxy::dequeue().await;
        let mut prompts = Vec::new();
        for pending in taken {
            let delivered = pending
                .settle(Response::Delivered {
                    r#type: Default::default(),
                })
                .await;
            if delivered {
                prompts.push(pending.prompt.clone());
            }
        }
        prompts
    }

    /// The folded messages whose fold happened at or before `at`,
    /// in fold order, removed: the runner asks with each tool
    /// completion's timestamp, and yields them right after that
    /// tool response.
    pub async fn deliveries_before(&self, at: f64) -> Vec<String> {
        let mut state = self.state.lock().await;
        let delivered = std::mem::take(&mut state.delivered);
        let (due, later): (Vec<_>, Vec<_>) =
            delivered.into_iter().partition(|delivery| delivery.at <= at);
        state.delivered = later;
        due.into_iter().map(|delivery| delivery.prompt).collect()
    }

    /// The run is over: everything pending is `missed`, and so is
    /// everything that arrives later.
    pub async fn close(&self) {
        let taken = {
            let mut state = self.state.lock().await;
            state.closed = true;
            std::mem::take(&mut state.pending)
        };
        for pending in taken {
            pending
                .settle(Response::Missed {
                    r#type: Default::default(),
                })
                .await;
        }
    }
}

/// Closes the queue however the run ends — dropped at the end of
/// the graceful path, where closing is already done, or by an
/// unwind, where it is not.
pub struct CloseOnDrop;

impl Drop for CloseOnDrop {
    fn drop(&mut self) {
        tokio::spawn(QUEUE.close());
    }
}
