//! The tree of identities.

use std::sync::Arc;

use dashmap::DashMap;

use super::Identity;
use crate::config::volumes::{Fixed, Store};

/// Every identity that has asked about its volumes, by identity.
///
/// Starts empty and reads nothing: an identity enters the tree the
/// first time it is named, and its volumes are read from the stores
/// then, once. Each identity is its own entry with its own lock, so
/// two identities never wait on each other.
#[derive(Debug, Default)]
pub struct Cache {
    identities: DashMap<String, Arc<Identity>>,
}

impl Cache {
    /// The identity's volumes, read from `stores` and `fixed` the first
    /// time this identity is named and held from then on. Every caller
    /// that names the identity while its first load runs waits for
    /// that load rather than starting another.
    pub async fn identity(&self, stores: &[Store], fixed: &[Fixed], client_identity: &str) -> Arc<Identity> {
        let identity = Arc::clone(
            &self
                .identities
                .entry(client_identity.to_string())
                .or_insert_with(|| Arc::new(Identity::new(client_identity))),
        );
        identity.load(stores, fixed).await;
        identity
    }

    /// Drop what is held for the identity, so the next time it is named
    /// its volumes are read from the stores again. A caller holding an
    /// [`Identity`] keeps it; only the tree forgets.
    pub fn forget(&self, client_identity: &str) {
        self.identities.remove(client_identity);
    }
}
