//! One token's origin, answered.

use std::sync::Arc;

use crate::host::image_manager::{Attempt, Store};
use crate::protocol::image_source::{ImageSource, Manifest};

use super::Digest;

/// Where a token's repository is served from (see
/// [`image_registry`](super), rules 3 and 4).
#[derive(Debug, Clone)]
pub enum Origin {
    /// The Store: its blobs, and nothing else.
    Store,
    /// A run's caller, read through: what the Store holds is served
    /// from it, and what it lacks is fetched from the caller, verified,
    /// written into the Store pinned for `attempt`, and served.
    Client {
        /// The caller's store over the run scope.
        source: ImageSource,
        /// The attempt every blob written is pinned for.
        attempt: Attempt,
    },
}

/// A token's repository: its origin.
#[derive(Debug)]
pub struct Repository {
    origin: Origin,
}

impl Repository {
    pub fn new(origin: Origin) -> Self {
        Repository { origin }
    }

    /// The manifest under `digest`: the Store's; else, for a caller's
    /// token, the caller's, verified and written into the Store.
    /// `None` for one that cannot be had, or whose bytes are not the
    /// digest's.
    pub async fn manifest(&self, store: &Arc<Store>, digest: &Digest) -> Option<Manifest> {
        if let Origin::Client { attempt, .. } = &self.origin {
            store.pin(*attempt, digest).await;
        }
        if let Some(manifest) = store.manifest(digest).await {
            return Some(manifest);
        }
        let Origin::Client { source, attempt } = &self.origin else {
            return None;
        };
        let manifest = source.manifest(&digest.to_string()).await?;
        if !store.write_bytes(*attempt, digest, &manifest.body).await.ok()? {
            return None;
        }
        store.manifest(digest).await
    }

    /// The size of the blob under `digest` in the Store, once it is
    /// there: for a caller's token, fetched from the caller first when
    /// the Store lacks it, verified and written. `None` for a blob
    /// that cannot be had.
    pub async fn blob(&self, store: &Arc<Store>, digest: &Digest) -> Option<u64> {
        let Origin::Client { source, attempt } = &self.origin else {
            return store.blob_size(digest).await;
        };
        if let Some(size) = store.hold(*attempt, digest).await {
            return Some(size);
        }
        let pieces = source.blob(&digest.to_string()).await?;
        store.write_stream(*attempt, digest, pieces).await.ok().flatten()
    }
}
