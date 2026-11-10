//! The container's one slot, between the HTTP door and the run.
//!
//! [`QUEUE`] is what the two sides share: the `/enqueue` handler
//! fills the slot with the proxy's batch — one message, however many
//! the proxy joined into it — and the `/dequeue` handler empties it,
//! the run takes it at each turn's end — the harness cannot be
//! steered mid-turn, so that is the one seam — as the next turn's one
//! input, and the batch's HTTP response waits on the fate whoever
//! acted sends — the SDK's own [`Fate`], because the fate IS the
//! response. The queue is the proxy's; this holds at most one
//! message, so a second `/enqueue` while it is full is [`Occupied`].
//! A true global, because the container runs one loop at a time — the
//! door enforces that — and one run has one queue: [`Queue::open`]
//! makes it the next run's, and every close names the run it belongs
//! to, so a close that arrives late cannot touch the run that came
//! after.

use diverge_sdk::container_proxy::inside::agent::enqueue::Fate;
use diverge_sdk::container_proxy::inside::agent::run::request::Message;
use tokio::sync::Mutex;
use tokio::sync::oneshot;

/// The slot.
pub static QUEUE: Queue = Queue {
    state: Mutex::const_new(State {
        pending: None,
        closed: true,
        generation: 0,
    }),
};

/// The one message waiting in the slot: the proxy's batch, and the
/// wire back to the caller still holding the `/enqueue` response
/// open.
pub struct Pending {
    /// The batch, in enqueue order, each message under its own key:
    /// taken as ONE user turn, the user parts of each under its key.
    pub messages: Vec<Message>,
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

/// A second `/enqueue` while the slot is full: refused, `409`
/// `{"kind":"occupied"}`, and the slot left as it was.
pub struct Occupied;

/// The slot itself: a [`Mutex`] around the pending message, the
/// closed flag and the run's number, and nothing else. Every
/// operation is a few pushes and takes — nothing awaits while
/// holding the lock; the waiting, which is the whole substance of an
/// enqueue, happens on the [`oneshot`] outside it.
pub struct Queue {
    state: Mutex<State>,
}

/// What the lock protects.
struct State {
    /// The message not yet taken, if the slot is full.
    pending: Option<Pending>,
    /// Whether the run is over. A closed queue takes nothing and
    /// holds nothing: enqueues answer missed immediately.
    ///
    /// The flag is only ever set INSIDE the same critical section
    /// that proves the queue empty — [`Queue::take_or_close`] — or
    /// that empties it — [`Queue::close`]. That is what makes "an
    /// enqueue that will never be delivered" impossible: there is no
    /// instant between the loop's last look and the closing in which
    /// one could land. Closed to begin with: no run, no queue.
    closed: bool,
    /// Which run the queue is: counted up by [`Queue::open`], and
    /// quoted by every [`Queue::close`], so a close spawned by a run
    /// that has ended — the run's teardown cannot await, so its
    /// close runs later — is a no-op once the next run has opened.
    generation: u64,
}

impl Queue {
    /// The next run's queue: open, and one generation on. Called by
    /// the run before anything of it can fail, so that everything
    /// enqueued from here on is this run's to take — or, if the run
    /// never gets as far as a loop, to miss by closing.
    pub async fn open(&self) -> u64 {
        let mut state = self.state.lock().await;
        state.closed = false;
        state.generation += 1;
        state.generation
    }

    /// Fill the slot, and get the wire its fate will arrive on — or
    /// [`Occupied`], the slot already full, which the proxy never
    /// causes: it offers nothing more until the first is answered.
    ///
    /// On a closed queue the fate is already known — the run is
    /// over, the message is missed — and the returned receiver
    /// resolves immediately.
    pub async fn enqueue(&self, messages: Vec<Message>) -> Result<oneshot::Receiver<Fate>, Occupied> {
        let (sender, receiver) = oneshot::channel();
        let mut state = self.state.lock().await;
        if state.closed {
            drop(state);
            let _ = sender.send(Fate::Missed);
        } else if state.pending.is_some() {
            return Err(Occupied);
        } else {
            state.pending = Some(Pending {
                messages,
                fate: sender,
            });
        }
        Ok(receiver)
    }

    /// Empty the slot, answering its message dequeued. Answers
    /// whether there was anything to withdraw.
    ///
    /// One take under the lock, and the fate sent after it is
    /// released. Naive about the closed flag, deliberately: a closed
    /// queue is an empty slot, so the honest answer falls out for
    /// free; and a message the loop already took is not here to be
    /// withdrawn.
    pub async fn dequeue(&self) -> bool {
        let taken = self.state.lock().await.pending.take();
        let any = taken.is_some();
        if let Some(pending) = taken {
            let _ = pending.fate.send(Fate::Dequeued);
        }
        any
    }

    /// The turn's end, the run's one look: take the slot's message,
    /// or — finding none — close the queue in the same breath.
    ///
    /// One critical section for both, and that is the whole point.
    /// The run only ends when this returns empty, and by then the
    /// closed flag is already set under the same lock hold that
    /// proved the emptiness — an enqueue arriving a nanosecond later
    /// finds the queue closed and is missed honestly, instead of
    /// waiting on a run that has already decided to end. A message
    /// taken opens another turn; the taker owes it its fate, by
    /// [`Pending::deliver`].
    pub async fn take_or_close(&self) -> Option<Pending> {
        let mut state = self.state.lock().await;
        if state.pending.is_none() {
            state.closed = true;
        }
        state.pending.take()
    }

    /// The run is over, however it got that way: the slot's message,
    /// if any, is missed, and so is everything that arrives after —
    /// until the next run opens. A close naming a run that is not
    /// the current one is a close that arrived late, and does
    /// nothing: the queue it meant to close is already gone.
    pub async fn close(&self, generation: u64) {
        let taken = {
            let mut state = self.state.lock().await;
            if state.generation != generation {
                return;
            }
            state.closed = true;
            state.pending.take()
        };
        if let Some(pending) = taken {
            let _ = pending.fate.send(Fate::Missed);
        }
    }
}
