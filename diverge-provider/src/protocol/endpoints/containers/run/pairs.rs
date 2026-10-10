//! Database connections whose caller half has not opened yet.

use std::collections::HashMap;

use tokio::sync::Mutex;

/// The proxy's id for each database connection, kept under the id
/// this end minted for the caller.
///
/// A connection is two ids: the proxy mints one and announces it on
/// the begin scope; this end mints another and opens the provider's
/// half on the run scope with it, because the caller's numbering and
/// the proxy's mean nothing to each other. When the caller opens its
/// half quoting this end's id, [`take`](Self::take) hands back the
/// proxy's, and this end opens its own half on the begin scope with
/// that. Nothing is buffered here: the proxy holds the driver's bytes
/// until this end asks for them.
#[derive(Debug)]
pub(crate) struct Pairs {
    waiting: Mutex<Waiting>,
}

/// What the lock protects: the next id, and the proxy's id under
/// each of this end's.
#[derive(Debug)]
struct Waiting {
    next: u32,
    by_caller_id: HashMap<u32, u32>,
}

impl Pairs {
    pub(crate) fn new() -> Self {
        Pairs {
            waiting: Mutex::new(Waiting {
                next: 1,
                by_caller_id: HashMap::new(),
            }),
        }
    }

    /// Mint an id for the caller, remembering the proxy's under it.
    pub(crate) async fn open(&self, proxy_id: u32) -> u32 {
        let mut waiting = self.waiting.lock().await;
        let caller_id = waiting.next;
        waiting.next = waiting.next.wrapping_add(1);
        waiting.by_caller_id.insert(caller_id, proxy_id);
        caller_id
    }

    /// The proxy's id for the connection the caller is opening its
    /// half of, once: [`None`] for an id this end did not mint, or a
    /// half already taken.
    pub(crate) async fn take(&self, caller_id: u32) -> Option<u32> {
        self.waiting.lock().await.by_caller_id.remove(&caller_id)
    }

    /// The provider's half is over before the caller opened its own:
    /// a half opened later finds nothing.
    pub(crate) async fn forget(&self, caller_id: u32) {
        self.waiting.lock().await.by_caller_id.remove(&caller_id);
    }
}
