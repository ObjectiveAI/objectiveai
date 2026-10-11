//! The store that serves the caller's layouts.

use std::sync::Arc;

use bytes::Bytes;
use tokio::fs;

use super::{Pieces, pieces};
use crate::host::image_manager::Store;
use crate::protocol::image_source::Manifest;
use crate::protocol::oci_store;

/// This provider's OCI store: the caller's own images, from their
/// layouts; nothing without the client image store.
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
    /// the repository path it was built as, less the `localhost/`.
    async fn holds(&self, client_identity: &str, digest: &str) -> Option<String> {
        let store = self.store.as_ref()?;
        let (name, _) = store.find_digest(client_identity, digest).await?;
        let tag = store.tag(client_identity, &name);
        Some(tag.trim_start_matches("localhost/").to_string())
    }

    /// The layout's manifest, for the caller's image of that digest.
    async fn manifest(&self, client_identity: &str, digest: &str) -> Option<Manifest> {
        let store = self.store.as_ref()?;
        let (name, _) = store.find_digest(client_identity, digest).await?;
        let dir = store.layout_dir(client_identity, &name);
        let layout = Store::read_layout(&dir).await.ok()?;
        if layout.digest != digest {
            return None;
        }
        let body = fs::read(Store::blob_path(&dir, digest)?).await.ok()?;
        Some(Manifest {
            media_type: layout.media_type,
            body: Bytes::from(body),
        })
    }

    type Blob = Pieces;

    /// The blob of that digest among the caller's layouts, the first
    /// layout holding it.
    async fn blob(&self, client_identity: &str, digest: &str) -> Option<Pieces> {
        let store = self.store.as_ref()?;
        for image in store.list(client_identity).await.ok()? {
            let dir = store.layout_dir(client_identity, &image.name);
            let Some(path) = Store::blob_path(&dir, digest) else {
                continue;
            };
            if fs::metadata(&path).await.is_ok() {
                return pieces(&path).await;
            }
        }
        None
    }
}
