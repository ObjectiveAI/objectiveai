//! Images: the daemon holds none.

use bytes::Bytes;
use diverge_sdk::provider::client::{Manifest, OciStore};
use futures_util::stream;

use super::Answerer;

/// The daemon keeps no image: a provider asked whether the caller
/// holds one is told no, and pulls from its own registries.
impl OciStore for Answerer {
    async fn holds(&self, _: &str) -> Option<String> {
        None
    }

    type Blob = stream::Empty<Bytes>;

    async fn manifest(&self, _: &str) -> Option<Manifest> {
        None
    }

    async fn blob(&self, _: &str) -> Option<Self::Blob> {
        None
    }
}
