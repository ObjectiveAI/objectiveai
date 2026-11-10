//! The container's queue, between the HTTP door and the run.
//!
//! [`QUEUE`] is what the two sides share: the `/enqueue` handler
//! appends the proxy's batch — however many messages the proxy joined
//! into it — and answers at once, and the run takes everything
//! waiting at each turn's end — the harness cannot be steered
//! mid-turn, so that is the one seam — as the next turn's ONE input,
//! each message's user parts under its own key. The `/enqueue`'s
//! `204` is the messages' delivery: nothing waits on the run, and
//! nothing withdraws a message once accepted. A true global, because
//! the container runs one loop at a time — the door enforces that —
//! and one run has one queue: [`Queue::open`] makes it the next
//! run's, and every close names the run it belongs to, so a close
//! that arrives late cannot touch the run that came after.

use diverge_sdk::container_proxy::inside::agent::run::request::Message;
use tokio::sync::Mutex;

/// The queue.
pub static QUEUE: Queue = Queue {
    state: Mutex::const_new(State {
        messages: Vec::new(),
        closed: true,
        generation: 0,
    }),
};

/// An `/enqueue` after the run closed: refused, `409`
/// `{"kind":"closed"}`, nothing accepted — the proxy keeps the batch
/// and starts the next run on it.
pub struct Closed;

/// The queue itself: a [`Mutex`] around the messages waiting, the
/// closed flag and the run's number, and nothing else. Every
/// operation is a few pushes and takes — nothing awaits while
/// holding the lock, and nothing awaits the run at all.
pub struct Queue {
    state: Mutex<State>,
}

/// What the lock protects.
struct State {
    /// The messages accepted and not yet taken, in the order they
    /// were enqueued.
    messages: Vec<Message>,
    /// Whether the run is over. A closed queue takes nothing and
    /// holds nothing: enqueues are refused [`Closed`] at once.
    ///
    /// The flag is only ever set INSIDE the same critical section
    /// that proves the queue empty — [`Queue::take_or_close`] — or
    /// that empties it — [`Queue::close`]. That is what makes "an
    /// accepted message no run will take" impossible on the graceful
    /// path: there is no instant between the run's last look and the
    /// closing in which one could land. Closed to begin with: no run,
    /// no queue.
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
    /// enqueued from here on is this run's to take.
    pub async fn open(&self) -> u64 {
        let mut state = self.state.lock().await;
        state.closed = false;
        state.generation += 1;
        state.generation
    }

    /// Accept the batch, behind whatever already waits — or
    /// [`Closed`], the run over, and nothing accepted.
    pub async fn enqueue(&self, messages: Vec<Message>) -> Result<(), Closed> {
        let mut state = self.state.lock().await;
        if state.closed {
            return Err(Closed);
        }
        state.messages.extend(messages);
        Ok(())
    }

    /// The turn's end, the run's one look: take everything waiting,
    /// or — finding nothing — close the queue in the same breath.
    ///
    /// One critical section for both, and that is the whole point.
    /// The run only ends when this returns empty, and by then the
    /// closed flag is already set under the same lock hold that
    /// proved the emptiness — an enqueue arriving a nanosecond later
    /// finds the queue closed and is refused honestly, instead of
    /// being accepted by a run that has already decided to end. What
    /// is taken opens another turn.
    pub async fn take_or_close(&self) -> Option<Vec<Message>> {
        let mut state = self.state.lock().await;
        if state.messages.is_empty() {
            state.closed = true;
            return None;
        }
        Some(std::mem::take(&mut state.messages))
    }

    /// The run is over, however it got that way: the queue is closed
    /// and emptied — what it accepted and no turn took ends with the
    /// run — and everything that arrives after is refused, until the
    /// next run opens. A close naming a run that is not the current
    /// one is a close that arrived late, and does nothing: the queue
    /// it meant to close is already gone.
    pub async fn close(&self, generation: u64) {
        let mut state = self.state.lock().await;
        if state.generation != generation {
            return;
        }
        state.closed = true;
        state.messages.clear();
    }
}
