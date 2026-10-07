//! Connections announced and waiting for their other half.

use std::collections::HashMap;

use tokio::sync::{Mutex, oneshot};

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
/// Every operation here is a map lookup and a send on a [`oneshot`],
/// neither of which yields, so the lock is taken and let go within
/// one call and no waiter holds it while suspended.
pub struct Pairs {
    pending: Mutex<Pending>,
}

/// What the lock protects: the next id, and the waiter under each.
struct Pending {
    next: u32,
    waiters: HashMap<u32, oneshot::Sender<u32>>,
}

impl Pairs {
    pub fn new() -> Self {
        Pairs {
            pending: Mutex::new(Pending {
                // From one: unique among the connections not yet
                // paired is all that is needed, and never reused is
                // the simplest way to be that.
                next: 1,
                waiters: HashMap::new(),
            }),
        }
    }

    /// A fresh id, and where the server's half arrives.
    ///
    /// The id is registered before this returns, so a half that
    /// arrives the instant the ask goes out finds its waiter: the ask
    /// cannot be sent until the caller holds the id, and the id is in
    /// the map before the ask is built.
    pub async fn announce(&self) -> (u32, oneshot::Receiver<u32>) {
        let mut pending = self.pending.lock().await;
        let id = pending.next;
        pending.next = pending.next.wrapping_add(1);
        let (sender, receiver) = oneshot::channel();
        pending.waiters.insert(id, sender);
        (id, receiver)
    }

    /// The server's half arrived for `id`, on `channel`: hand it over.
    /// `false` is an id nobody is waiting on — one never announced,
    /// one whose half came already, or one whose connection is gone.
    pub async fn pair(&self, id: u32, channel: u32) -> bool {
        let Some(sender) = self.pending.lock().await.waiters.remove(&id) else {
            return false;
        };
        sender.send(channel).is_ok()
    }

    /// The announcement is withdrawn: the ask ended before the half
    /// came.
    pub async fn forget(&self, id: u32) {
        self.pending.lock().await.waiters.remove(&id);
    }
}
