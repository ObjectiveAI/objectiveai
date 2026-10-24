//! The one-connection-per-identity rule.

use std::collections::HashSet;

use diverge_sdk::shared::Identity;
use tokio::sync::Mutex;

/// The identities a connection is held under now: at most one
/// connection per identity at a time, whichever end dialled. The
/// handshake takes its peer's identity as it verifies it — a held
/// identity is not a valid authentication — and the connection lets
/// it go when it ends.
#[derive(Debug, Default)]
pub struct Peers {
    held: Mutex<HashSet<Identity>>,
}

impl Peers {
    /// Take `identity` for a connection: `false`, and nothing taken,
    /// when another connection holds it.
    pub async fn take(&self, identity: &Identity) -> bool {
        self.held.lock().await.insert(identity.clone())
    }

    /// Let `identity` go, its connection ended.
    pub async fn release(&self, identity: &Identity) {
        self.held.lock().await.remove(identity);
    }
}
