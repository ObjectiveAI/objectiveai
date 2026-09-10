//! The database connections in flight, by the id this end minted.

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

use bytes::Bytes;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

/// A connection is a pair of channels — see
/// [`postgres`](crate::shared::containers::postgres) — opened at
/// different moments by different ends: this end's half when the
/// container's driver connects, the caller's half once it has dialled
/// its database and answered. What the container writes in between
/// has to wait somewhere, and this is where: a queue per connection,
/// filled by the relay from the moment the pair is asked for, taken
/// by the caller's half when it opens.
pub(crate) struct Pairs {
    next: AtomicU32,
    waiting: Mutex<HashMap<u32, UnboundedReceiver<Bytes>>>,
}

impl Pairs {
    pub(crate) fn new() -> Self {
        Pairs {
            next: AtomicU32::new(1),
            waiting: Mutex::new(HashMap::new()),
        }
    }

    /// A fresh connection id, and the queue the container's bytes go
    /// into until the caller's half takes it.
    pub(crate) fn open(&self) -> (u32, UnboundedSender<Bytes>) {
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        let (sender, receiver) = mpsc::unbounded_channel();
        self.lock().insert(id, receiver);
        (id, sender)
    }

    /// The caller's half opened: its queue, or `None` for an id this
    /// end never minted, or one already taken — a second half for one
    /// connection, which is answered with nothing.
    pub(crate) fn take(&self, id: u32) -> Option<UnboundedReceiver<Bytes>> {
        self.lock().remove(&id)
    }

    /// The connection is over and its half was never taken: drop the
    /// queue, so what it holds goes with it.
    pub(crate) fn forget(&self, id: u32) {
        self.lock().remove(&id);
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<u32, UnboundedReceiver<Bytes>>> {
        self.waiting.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl std::fmt::Debug for Pairs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pairs").field("waiting", &self.lock().len()).finish()
    }
}
