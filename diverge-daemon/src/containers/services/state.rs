//! What is live for one service, across its runs.

use std::time::Instant;

use tokio::sync::{Mutex, watch};
use tokio::task::AbortHandle;

/// A service as the daemon keeps it, run or no run: whether any agent
/// is attached to it, when its attachments last changed, and the two
/// tasks that keep it — the supervisor, which runs its container for
/// as long as it is held, and the clock, which retires it once it is
/// not. Lives from the service's first keep to its retirement, across
/// every run of its container.
#[derive(Debug)]
pub struct State {
    /// Whether any agent is attached: `true` while one is, kept in
    /// step with the attachments by [`held`](super::held). What the
    /// supervisor waits on to run, and the clock waits on to stop.
    pub held: watch::Sender<bool>,
    /// When the attachments last changed: what the clock counts from.
    pub touched: watch::Sender<Instant>,
    /// The supervisor's task, once started.
    pub supervisor: Mutex<Option<AbortHandle>>,
    /// The clock's task, once started.
    pub clock: Mutex<Option<AbortHandle>>,
}

impl State {
    /// A service held up, as of now, with neither task started.
    pub fn new() -> State {
        State {
            held: watch::channel(true).0,
            touched: watch::channel(Instant::now()).0,
            supervisor: Mutex::new(None),
            clock: Mutex::new(None),
        }
    }
}

impl Default for State {
    fn default() -> Self {
        State::new()
    }
}
