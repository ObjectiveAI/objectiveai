//! The daemons accepting connections through this provider, by
//! identity.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{Mutex, oneshot, watch};

use diverge_sdk::wire::server::scope_handle::ScopeHandle;

/// Every daemon holding a `daemons::accept` scope on this provider,
/// by the identity its connection is authorized under — what a
/// `daemons::connect` naming that identity has to find, on whatever
/// connection it arrives.
///
/// One per provider, shared across every connection's
/// [`handle`](super::handle::handle), as the [`Directory`](super::directory::Directory)
/// is: a daemon connector names a daemon that holds its accept scope
/// on another socket entirely. One accept scope per identity: a second
/// under an identity that holds one is refused.
pub struct Acceptors {
    accepting: Mutex<HashMap<Arc<str>, Arc<Accepting>>>,
}

/// One daemon's accept scope, and the connections being paired on it.
pub struct Accepting {
    /// The accept scope: where the provider's half of every connection
    /// is opened, and where the acceptor opens its own.
    pub scope: Arc<ScopeHandle>,
    /// The next connection id.
    next: Mutex<u32>,
    /// Every connection whose acceptor's half has not opened yet, by
    /// id: what hands the half's channel number to the connect
    /// handler waiting for it.
    waiting: Mutex<HashMap<u32, oneshot::Sender<u32>>>,
    /// `true` once the accept scope has ended: every connection
    /// relayed on it ends with it.
    ended: watch::Sender<bool>,
}

impl Acceptors {
    pub fn new() -> Self {
        Acceptors {
            accepting: Mutex::new(HashMap::new()),
        }
    }

    /// The identity is accepting on `scope` from now until
    /// [`release`](Self::release): `None` when it accepts already, and
    /// nothing changed.
    pub(crate) async fn take(&self, identity: &str, scope: Arc<ScopeHandle>) -> Option<Arc<Accepting>> {
        let mut accepting = self.accepting.lock().await;
        if accepting.contains_key(identity) {
            return None;
        }
        let entry = Arc::new(Accepting {
            scope,
            next: Mutex::new(1),
            waiting: Mutex::new(HashMap::new()),
            ended: watch::channel(false).0,
        });
        accepting.insert(Arc::from(identity), Arc::clone(&entry));
        Some(entry)
    }

    /// The identity's accept scope is over: nothing finds it any more,
    /// and every connection relayed on it hears so.
    pub(crate) async fn release(&self, identity: &str) {
        if let Some(entry) = self.accepting.lock().await.remove(identity) {
            entry.ended.send_replace(true);
        }
    }

    /// The daemon accepting under `identity`, if one is.
    pub(crate) async fn lookup(&self, identity: &str) -> Option<Arc<Accepting>> {
        self.accepting.lock().await.get(identity).cloned()
    }
}

impl Accepting {
    /// Mint an id for a connection, and the receiver that is handed
    /// the acceptor's half when it opens.
    pub(crate) async fn open(&self) -> (u32, oneshot::Receiver<u32>) {
        let id = {
            let mut next = self.next.lock().await;
            let id = *next;
            *next = next.wrapping_add(1);
            id
        };
        let (sender, receiver) = oneshot::channel();
        self.waiting.lock().await.insert(id, sender);
        (id, receiver)
    }

    /// The acceptor opened its half of the connection under `id`, on
    /// `channel` of the accept scope: handed to whoever waits for it.
    /// An id nobody waits for — never minted, or its connection over —
    /// is `false`, and the half is answered with nothing.
    pub(crate) async fn deliver(&self, id: u32, channel: u32) -> bool {
        match self.waiting.lock().await.remove(&id) {
            Some(sender) => sender.send(channel).is_ok(),
            None => false,
        }
    }

    /// The connection under `id` is over before its half opened: a
    /// half opened later finds nothing.
    pub(crate) async fn forget(&self, id: u32) {
        self.waiting.lock().await.remove(&id);
    }

    /// `true` once the accept scope has ended. A receiver whose sender
    /// is gone reads the same.
    pub(crate) fn ended(&self) -> watch::Receiver<bool> {
        self.ended.subscribe()
    }
}

impl Default for Acceptors {
    fn default() -> Self {
        Acceptors::new()
    }
}

impl std::fmt::Debug for Acceptors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug = f.debug_struct("Acceptors");
        match self.accepting.try_lock() {
            Ok(accepting) => debug.field("accepting", &accepting.len()),
            Err(_) => debug.field("accepting", &"locked"),
        };
        debug.finish()
    }
}
