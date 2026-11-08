//! Images: the daemon holds none itself, and answers from the other
//! providers.

use diverge_sdk::provider::client::{Manifest, OciStore};
use diverge_sdk::shared::containers::oci;
use diverge_sdk::shared::containers::request::Image;

use super::Answerer;
use crate::containers::images;

/// A provider asked whether the caller holds an image is answered
/// from the other providers connected: one that holds it is found,
/// and its manifests and blobs are pulled from there — see
/// [`images`](crate::containers::images), through the run's own
/// scopes. The provider asking is never asked, having looked already.
/// Asked for the proof of possession, the daemon relays it from a
/// provider that can supply the image, and hashes nothing itself.
impl OciStore for Answerer {
    async fn holds(&self, image: &Image) -> Option<String> {
        images::locate(&self.daemon, image, Some(&self.provider))
            .await
            .map(|_| images::name(&image.digest))
    }

    type Blob = images::Blob;

    async fn manifest(&self, image: &Image, digest: &str) -> Option<Manifest> {
        images::manifest(&self.daemon, &self.pulls, image, digest, &self.provider).await
    }

    async fn blob(&self, image: &Image, digest: &str) -> Option<Self::Blob> {
        images::blob(&self.daemon, &self.pulls, image, digest, &self.provider).await
    }

    async fn prove(&self, request: &oci::prove::request::Request) -> Option<[u8; 32]> {
        images::prove(&self.daemon, request, &self.provider).await
    }
}
