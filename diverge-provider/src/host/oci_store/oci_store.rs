//! The store that serves the caller's pool.

use diverge_sdk::shared::Identity;
use std::sync::Arc;

use crate::host::image_manager::{Pieces, Store, pool};
use crate::host::image_registry::Digest;
use crate::protocol::image_source::Manifest;
use crate::protocol::oci_store;

/// This provider's OCI store: the caller's own images, from its
/// pool; nothing without the client image store.
#[derive(Debug)]
pub struct OciStore {
    /// The client image store; `None` where the configuration has
    /// none.
    store: Option<Arc<Store>>,
}

impl OciStore {
    /// Over `store`.
    pub fn new(store: Option<Arc<Store>>) -> Self {
        OciStore { store }
    }
}

impl oci_store::OciStore for OciStore {
    /// Held where the caller built an image of that digest: under
    /// `<hash>/<name>`, the repository path the provider's own
    /// registry serves it at.
    async fn holds(&self, client_identity: &Identity, digest: &str) -> Option<String> {
        let store = self.store.as_ref()?;
        let (name, _) = store.find_digest(client_identity, digest).await?;
        Some(format!("{}/{name}", Store::hash(client_identity)))
    }

    /// The manifest of the caller's image of that digest, from its
    /// pool: the record found first, so that a manifest is served
    /// under a digest the caller built and no other.
    async fn manifest(&self, client_identity: &Identity, digest: &str) -> Option<Manifest> {
        let store = self.store.as_ref()?;
        store.find_digest(client_identity, digest).await?;
        let parsed = Digest::parse(digest)?;
        pool::manifest(&store.pool_dir(client_identity), &parsed).await
    }

    type Blob = Pieces;

    /// The blob of that digest in the caller's pool, and in no other
    /// identity's.
    async fn blob(&self, client_identity: &Identity, digest: &str) -> Option<Pieces> {
        let store = self.store.as_ref()?;
        let parsed = Digest::parse(digest)?;
        pool::blob(&store.pool_dir(client_identity), &parsed).await
    }
}
