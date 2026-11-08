//! The store that serves the caller's Store.

use diverge_sdk::shared::Identity;
use std::sync::Arc;

use crate::host::image_manager::{Pieces, Store};
use crate::host::image_registry::Digest;
use crate::protocol::image_source::Manifest;
use crate::protocol::oci_store;

/// The label a `has` is answered with: the name a puller writes into
/// its reference, never stored and never read.
const LABEL: &str = "image";

/// This provider's OCI store: the caller's own images, from the
/// Store; nothing without one.
#[derive(Debug)]
pub struct OciStore {
    /// The Store; `None` where the configuration has none.
    store: Option<Arc<Store>>,
}

impl OciStore {
    /// Over `store`.
    pub fn new(store: Option<Arc<Store>>) -> Self {
        OciStore { store }
    }
}

impl oci_store::OciStore for OciStore {
    /// Held where the caller owns the digest, under the fixed label
    /// `image`.
    async fn holds(&self, client_identity: &Identity, digest: &str) -> Option<String> {
        let store = self.store.as_ref()?;
        store.owns(client_identity, digest).await.then(|| LABEL.to_string())
    }

    /// A manifest the caller's Store reaches — an image it owns, or a
    /// platform manifest of an index it owns — with the media type it
    /// is served as.
    async fn manifest(&self, client_identity: &Identity, digest: &str) -> Option<Manifest> {
        let store = self.store.as_ref()?;
        let parsed = Digest::parse(digest)?;
        if !store.reaches_owned(client_identity, &parsed).await {
            return None;
        }
        store.manifest(&parsed).await
    }

    type Blob = Pieces;

    /// A blob the caller's Store reaches, and no other: one copy of a
    /// blob serves every owner, so what is served is decided by the
    /// caller's records alone.
    async fn blob(&self, client_identity: &Identity, digest: &str) -> Option<Pieces> {
        let store = self.store.as_ref()?;
        let parsed = Digest::parse(digest)?;
        if !store.reaches_owned(client_identity, &parsed).await {
            return None;
        }
        store.pieces(&parsed).await
    }
}
