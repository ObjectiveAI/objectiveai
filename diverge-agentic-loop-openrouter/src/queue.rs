//! The container's one queue, between the HTTP door and the loop.
//!
//! [`QUEUE`] is what the two sides share: the `/enqueue` and
//! `/dequeue` handlers put messages in and clear them, the loop
//! takes them at its seams, and every message's HTTP response waits
//! on the fate whoever acted sends — the SDK's own [`Fate`], because
//! the fate IS the response. A true global, because the container is
//! one run — the door already enforces that — and one run has one
//! queue.

use diverge_provider_sdk::container_proxy::agent::enqueue::Fate;
use tokio::sync::Mutex;
use tokio::sync::oneshot;

/// The queue.
pub static QUEUE: Queue = Queue {
    state: Mutex::const_new(State {
        pending: Vec::new(),
        closed: false,
    }),
};

/// One message waiting in the queue: its text, and the wire back to
/// the caller still holding the `/enqueue` response open.
pub struct Pending {
    /// The message's text.
    pub prompt: String,
    /// Where the fate goes — the SDK's own [`Fate`], because the fate
    /// IS the response and a second vocabulary for it would be a
    /// second thing to keep agreeing. Consumed by exactly one of
    /// [`Pending::deliver`], the dequeue that clears it, or the close
    /// that misses it.
    fate: oneshot::Sender<Fate>,
}

impl Pending {
    /// The loop took this message: send the fate.
    ///
    /// Ignoring the send's outcome is deliberate — a caller that
    /// dropped its `/enqueue` request stopped listening, and the
    /// message is delivered whether or not anybody hears it said.
    pub fn deliver(self) {
        let _ = self.fate.send(Fate::Delivered);
    }
}

/// The queue itself: a [`Mutex`] around the pending messages and the
/// closed flag, and nothing else. Every operation is a few pushes
/// and takes — nothing awaits while holding the lock; the waiting,
/// which is the whole substance of an enqueue, happens on the
/// [`oneshot`] outside it.
pub struct Queue {
    state: Mutex<State>,
}

/// What the lock protects.
struct State {
    /// Messages not yet taken, in arrival order.
    pending: Vec<Pending>,
    /// Whether the run is over. A closed queue takes nothing and
    /// holds nothing: enqueues answer missed immediately.
    ///
    /// The flag is only ever set INSIDE the same critical section
    /// that proves the queue empty — [`Queue::take_or_close`] — or
    /// that empties it — [`Queue::close`]. That is what makes "an
    /// enqueue that will never be delivered" impossible: there is no
    /// instant between the loop's last look and the closing in which
    /// one could land.
    closed: bool,
}

impl Queue {
    /// Put a message in, and get the wire its fate will arrive on.
    ///
    /// On a closed queue the fate is already known — the run is
    /// over, the message is missed — and the returned receiver
    /// resolves immediately.
    pub async fn enqueue(&self, prompt: String) -> oneshot::Receiver<Fate> {
        let (sender, receiver) = oneshot::channel();
        let mut state = self.state.lock().await;
        if state.closed {
            drop(state);
            let _ = sender.send(Fate::Missed);
        } else {
            state.pending.push(Pending {
                prompt,
                fate: sender,
            });
        }
        receiver
    }

    /// Withdraw everything pending, answering each message
    /// dequeued. Answers whether there was anything to withdraw.
    ///
    /// Naive about the closed flag, deliberately: a closed queue is
    /// an empty queue, so the honest answer falls out for free.
    pub async fn dequeue(&self) -> bool {
        let taken = {
            let mut state = self.state.lock().await;
            std::mem::take(&mut state.pending)
        };
        let any = !taken.is_empty();
        for pending in taken {
            let _ = pending.fate.send(Fate::Dequeued);
        }
        any
    }

    /// Take everything pending, for delivery — the tool seam's
    /// drain. The queue stays open; the taker owes each returned
    /// message its fate, by [`Pending::deliver`].
    pub async fn take(&self) -> Vec<Pending> {
        let mut state = self.state.lock().await;
        std::mem::take(&mut state.pending)
    }

    /// The loop's LAST look: take everything pending, or — finding
    /// nothing — close the queue in the same breath.
    ///
    /// One critical section for both, and that is the whole point.
    /// The loop only falls out when this returns empty, and by then
    /// the closed flag is already set under the same lock hold that
    /// proved the emptiness — an enqueue arriving a nanosecond later
    /// finds the queue closed and is missed honestly, instead of
    /// waiting on a loop that has already decided to end.
    pub async fn take_or_close(&self) -> Vec<Pending> {
        let mut state = self.state.lock().await;
        if state.pending.is_empty() {
            state.closed = true;
        }
        std::mem::take(&mut state.pending)
    }

    /// The run is over, however it got that way: everything still
    /// pending is missed, and so is everything that arrives after.
    pub async fn close(&self) {
        let taken = {
            let mut state = self.state.lock().await;
            state.closed = true;
            std::mem::take(&mut state.pending)
        };
        for pending in taken {
            let _ = pending.fate.send(Fate::Missed);
        }
    }
}

/// Closes [`QUEUE`] when dropped.
///
/// The loop holds one so that EVERY way its stream ends — the
/// graceful close already done by
/// [`take_or_close`](Queue::take_or_close), an error yielded, or the
/// consumer dropping the stream mid-run — leaves the queue closed
/// and every pending message answered. [`Drop`] cannot await, so the
/// closing rides a spawned task; the graceful path makes it a no-op.
pub struct CloseOnDrop;

impl Drop for CloseOnDrop {
    fn drop(&mut self) {
        tokio::spawn(QUEUE.close());
    }
}
