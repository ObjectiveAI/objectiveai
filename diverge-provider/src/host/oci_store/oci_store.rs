//! The store that holds nothing.

use bytes::Bytes;
use futures_util::stream;

use crate::protocol::image_source::Manifest;
use crate::protocol::oci_store;

/// This provider's OCI store: nothing held for anyone, every ask not
/// held.
#[derive(Debug, Default)]
pub struct OciStore;

impl oci_store::OciStore for OciStore {
    /// Not held: there is no store yet.
    async fn holds(&self, _client_identity: &str, _digest: &str) -> Option<String> {
        None
    }

    /// Not held: there is no store yet.
    async fn manifest(&self, _client_identity: &str, _digest: &str) -> Option<Manifest> {
        None
    }

    type Blob = stream::Empty<Bytes>;

    /// Not held: there is no store yet.
    async fn blob(&self, _client_identity: &str, _digest: &str) -> Option<Self::Blob> {
        None
    }
}
