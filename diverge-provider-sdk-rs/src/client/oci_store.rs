//! What a caller holds of an image it asked a provider to run.

use std::future::Future;

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
/// is once it has been saved anywhere, and what a caller has when it
/// holds an image it asks a provider to run.
///
/// A provider that would take the image from the caller asks first
/// whether the caller holds it, then runs the registry its runtime
/// pulls from and asks the caller by digest for whatever that
/// registry's store does not hold, over
/// [`oci`](crate::shared::containers::oci). The caller needs no HTTP
/// at all: it answers a manifest whole and a blob as a stream, and
/// answers nothing for a digest it does not hold — which the executor
/// sends as the empty finish, the wire's "could not serve". There is
/// no error vocabulary on this exchange; a store that cannot read
/// what it holds answers as if it did not. A caller that holds no
/// image at all answers `false` and `None` to everything, and a
/// provider that can get the image elsewhere runs it all the same.
pub trait OciStore: Send + Sync {
    /// Whether the image is held: its manifest under `digest`, as the
    /// run request named it by `name`.
    fn holds(&self, name: &str, digest: &str) -> impl Future<Output = bool> + Send;

    /// A blob's pieces, in order. Piece sizes are the store's; the
    /// executor re-splits at [`CHUNK_SIZE`](crate::CHUNK_SIZE).
    type Blob: Stream<Item = Bytes> + Send + 'static;

    /// The manifest under `digest`, or `None` for one not held.
    fn manifest(&self, digest: &str) -> impl Future<Output = Option<Manifest>> + Send;

    /// The blob under `digest`, or `None` for one not held.
    fn blob(&self, digest: &str) -> impl Future<Output = Option<Self::Blob>> + Send;
}
