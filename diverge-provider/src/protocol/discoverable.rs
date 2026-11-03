//! The identities connected to this provider that may be listed.

use std::collections::BTreeSet;

use diverge_sdk::shared::Identity;
use tokio::sync::Mutex;

/// Every identity connected to this provider whose authorization frame
/// carried the discover flag `1` — what a `discover` on any connection
/// answers.
///
/// One per provider, shared across every connection, as the
/// [`Acceptors`](super::acceptors::Acceptors) are. The rules:
///
/// 1. **Entered with its connection.** The host enters an identity
///    when a connection under it is past its handshake with the flag
///    `1` — whichever end dialled — and before the connection is
///    served.
/// 2. **Left with it.** The host lets it go when that connection ends.
///    An identity holds at most one connection to a provider, so one
///    entry is one connection.
/// 3. **Listed in order.** By issuer, then by subject — the set's own
///    order.
#[derive(Debug, Default)]
pub struct Discoverable {
    held: Mutex<BTreeSet<Identity>>,
}

impl Discoverable {
    pub fn new() -> Self {
        Discoverable::default()
    }

    /// `identity` is connected, and discoverable, until
    /// [`leave`](Self::leave).
    pub(crate) async fn enter(&self, identity: &Identity) {
        self.held.lock().await.insert(identity.clone());
    }

    /// `identity`'s connection is over: nothing lists it any more.
    pub(crate) async fn leave(&self, identity: &Identity) {
        self.held.lock().await.remove(identity);
    }

    /// Every discoverable identity connected now, in order.
    pub(crate) async fn list(&self) -> Vec<Identity> {
        self.held.lock().await.iter().cloned().collect()
    }
}
