//! What a caller holds of an image it asked a provider to run.

use std::future::Future;
use std::pin::Pin;

use bytes::Bytes;
use futures_util::Stream;

/// A manifest, as the store holds it: its media type and its bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    /// The manifest's media type, as the registry would have said it.
    pub media_type: String,
    /// The manifest, verbatim.
    pub body: Bytes,
}

/// A store of manifests and blobs indexed by digest — what an image
/// is once it has been saved anywhere, and what a caller needs to
/// have a provider run an [`Image::Client`](crate::shared::containers::request::Image::Client).
///
/// The provider runs the registry its runtime pulls from; whatever
/// that registry's store does not hold, the provider asks the caller
/// for by digest, over [`oci`](crate::shared::containers::oci). The
/// caller needs no HTTP at all: it answers a manifest whole and a
/// blob as a stream, and answers nothing for a digest it does not
/// hold — which the executor sends as the empty finish, the wire's
/// "could not serve". There is no error vocabulary on this exchange;
/// a store that cannot read what it holds answers as if it did not.
pub trait OciStore: Send + Sync {
    /// The manifest under `digest`, or `None` for one not held.
    fn manifest(&self, digest: &str) -> impl Future<Output = Option<Manifest>> + Send;

    /// The blob under `digest` as its pieces in order, or `None` for
    /// one not held. Piece sizes are the store's; the executor
    /// re-splits at [`CHUNK_SIZE`](crate::CHUNK_SIZE).
    fn blob(
        &self,
        digest: &str,
    ) -> impl Future<Output = Option<Pin<Box<dyn Stream<Item = Bytes> + Send + 'static>>>> + Send;
}
