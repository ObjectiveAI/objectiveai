//! Images: the daemon holds none itself, and answers from the other
//! providers.

use diverge_sdk::provider::client::{Manifest, OciStore};

use super::Answerer;
use crate::containers::images;

/// A provider asked whether the caller holds an image is answered
/// from the other providers connected: one that holds it is found,
/// and its manifests and blobs are pulled from there — see
/// [`images`](crate::containers::images). The provider asking is
/// never asked, having looked already.
impl OciStore for Answerer {
    async fn holds(&self, digest: &str) -> Option<String> {
        images::locate(&self.daemon, digest, Some(&self.provider))
            .await
            .map(|_| images::name(digest))
    }

    type Blob = images::Blob;

    async fn manifest(&self, digest: &str) -> Option<Manifest> {
        images::manifest(&self.daemon, digest, &self.provider).await
    }

    async fn blob(&self, digest: &str) -> Option<Self::Blob> {
        images::blob(&self.daemon, digest, &self.provider).await
    }
}
