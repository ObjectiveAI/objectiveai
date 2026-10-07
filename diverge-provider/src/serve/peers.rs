//! The peers connected now: one connection per identity, and one per
//! credential.

use std::collections::HashSet;

use sha2::{Digest as _, Sha256};
use tokio::sync::Mutex;

/// Every peer that dialled in and is connected now, under one lock
/// over both keys: no two connections hold one identity, and no two
/// hold one credential. A second connection that is admitted as an
/// identity a connection holds, or that presented a credential one
/// holds, is refused at its handshake, which reaches it as the close
/// and nothing else. The peers the provider dials itself take nothing
/// here: one task dials each, and the identity is the provider's own
/// choice.
#[derive(Debug, Default)]
pub struct Peers {
    held: Mutex<Held>,
}

/// What is held.
#[derive(Debug, Default)]
struct Held {
    /// The identities connected now.
    identities: HashSet<String>,
    /// The credentials connected through now, each as its digest.
    credentials: HashSet<[u8; 32]>,
}

impl Peers {
    /// Take the identity and the credential for a connection: `false`
    /// when a connection holds the identity, or one holds the
    /// credential, already, and nothing is taken; else both are, and
    /// are given back with [`release`](Self::release), on every path.
    pub async fn take(&self, identity: &str, credential: [u8; 32]) -> bool {
        let mut held = self.held.lock().await;
        if held.identities.contains(identity) || held.credentials.contains(&credential) {
            return false;
        }
        held.identities.insert(identity.to_string());
        held.credentials.insert(credential);
        true
    }

    /// The connection ended: both given back.
    pub async fn release(&self, identity: &str, credential: &[u8; 32]) {
        let mut held = self.held.lock().await;
        held.identities.remove(identity);
        held.credentials.remove(credential);
    }
}

/// A credential as it is held: its SHA-256, never its bytes.
pub fn digest(credential: &str) -> [u8; 32] {
    Sha256::digest(credential.as_bytes()).into()
}
