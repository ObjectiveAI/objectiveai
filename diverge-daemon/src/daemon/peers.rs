//! The providers connected now, one connection per identity and per
//! credential.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use diverge_sdk::wire::client::handle::Handle;
use tokio::sync::Notify;

/// Every provider the daemon holds a connection to, under one lock
/// over both keys: no two connections hold one identity, and no two
/// hold one credential — the hash of the key an incoming provider
/// presented. A second connection that collides on either is refused
/// at its attach and its socket closed without a word.
#[derive(Default)]
pub struct Peers {
    /// One slot per identity connected, or being admitted.
    pub slots: HashMap<Identity, Slot>,
    /// The credential hashes held by the incoming ones.
    pub credentials: HashSet<String>,
}

/// One provider's connection: the handle, once the provider answered
/// its version; the word that evicts it; and the credential it holds,
/// for an incoming one.
pub struct Slot {
    /// The caller's handle on the provider, filled once the version
    /// came back: what every request to a provider rides.
    pub handle: Option<Handle>,
    /// Told once when the daemon ends the connection itself — an
    /// incoming credential edited — on which the connection's own
    /// task closes the socket and gives the slot back.
    pub evict: Arc<Notify>,
    /// The hash of the key the provider presented, for an incoming
    /// provider; an outgoing one presents the daemon's.
    pub credential: Option<String>,
}

impl Peers {
    /// Take the slot for `identity`, holding `credential` with it:
    /// `None` when a connection holds the identity, or one holds the
    /// credential, already; else the slot's eviction word, and the
    /// slot is the caller's to fill and to give back.
    pub fn take(&mut self, identity: Identity, credential: Option<String>) -> Option<Arc<Notify>> {
        if self.slots.contains_key(&identity) {
            return None;
        }
        if credential.as_ref().is_some_and(|hash| self.credentials.contains(hash)) {
            return None;
        }
        let evict = Arc::new(Notify::new());
        if let Some(hash) = &credential {
            self.credentials.insert(hash.clone());
        }
        self.slots.insert(
            identity,
            Slot {
                handle: None,
                evict: Arc::clone(&evict),
                credential,
            },
        );
        Some(evict)
    }

    /// The slot given back, and the credential it held with it;
    /// `false` when there was none to give.
    pub fn release(&mut self, identity: &Identity) -> bool {
        let Some(slot) = self.slots.remove(identity) else {
            return false;
        };
        if let Some(hash) = slot.credential {
            self.credentials.remove(&hash);
        }
        true
    }
}
