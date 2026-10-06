//! Connections announced and waiting for their other half.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

use tokio::sync::oneshot;

/// The connections announced and not yet paired: each waits for the
/// server to open its half, quoting the id, and is handed the
/// channel it did.
///
/// One of these per KIND of pair — a Postgres connection and a daemon
/// connection are announced on different channels and number
/// independently — so that an id of one kind never resolves a waiter
/// of the other.
///
/// # The lock is never held across an await
///
/// It is a [`std::sync::Mutex`] on purpose: every operation here is a
/// map lookup and a send on a [`oneshot`], neither of which yields, so
/// a waiter never holds it while suspended and an async mutex would
/// buy nothing but the chance to hold one by accident.
pub struct Pairs {
    next: AtomicU32,
    pending: Mutex<HashMap<u32, oneshot::Sender<u32>>>,
}

impl Pairs {
    pub fn new() -> Self {
        Pairs {
            // From one: unique among the connections not yet paired is
            // all that is needed, and never reused is the simplest
            // way to be that.
            next: AtomicU32::new(1),
            pending: Mutex::new(HashMap::new()),
        }
    }

    /// A fresh id, and where the server's half arrives.
    ///
    /// The id is registered before this returns, so a half that
    /// arrives the instant the ask goes out finds its waiter: the ask
    /// cannot be sent until the caller holds the id, and the id is in
    /// the map before the ask is built.
    pub fn announce(&self) -> (u32, oneshot::Receiver<u32>) {
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        let (sender, receiver) = oneshot::channel();
        self.lock().insert(id, sender);
        (id, receiver)
    }

    /// The server's half arrived for `id`, on `channel`: hand it over.
    /// `false` is an id nobody is waiting on — one never announced,
    /// one whose half came already, or one whose connection is gone.
    pub fn pair(&self, id: u32, channel: u32) -> bool {
        let Some(sender) = self.lock().remove(&id) else {
            return false;
        };
        sender.send(channel).is_ok()
    }

    /// The announcement is withdrawn: the ask ended before the half
    /// came.
    pub fn forget(&self, id: u32) {
        self.lock().remove(&id);
    }

    /// The map, past a poisoning: a panic in a map lookup leaves
    /// nothing half-written, so the entries are still the entries.
    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<u32, oneshot::Sender<u32>>> {
        self.pending.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}
