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
/// registry's store does not hold, and last asks the caller to prove
/// it holds the image, over [`oci`](crate::shared::containers::oci).
/// An [`images::push`](crate::provider::endpoints::images::push) asks
/// the same, less the first question. The caller needs no HTTP
/// at all: it answers a manifest whole and a blob as a stream, and
/// answers nothing for a digest it does not hold — which the executor
/// sends as the empty finish, the wire's "could not serve". There is
/// no error vocabulary on this exchange; a store that cannot read
/// what it holds answers as if it did not. A caller that holds no
/// image at all answers `None` to everything, and a
/// provider that can get the image elsewhere runs it all the same.
/// A caller need not hold the bytes itself: one with images on other
/// providers answers from them, through
/// [`images::oci`](crate::provider::endpoints::images::oci), and the
/// provider asking never learns the difference.
pub trait OciStore: Send + Sync {
    /// Whether the image is held — its manifest under `digest` — and
    /// the repository path it is held under, which the provider's
    /// registry serves it as; `None` for one not held.
    fn holds(&self, digest: &str) -> impl Future<Output = Option<String>> + Send;

    /// A blob's pieces, in order. Piece sizes are the store's; the
    /// executor re-splits at [`CHUNK_SIZE`](crate::CHUNK_SIZE).
    type Blob: Stream<Item = Bytes> + Send + 'static;

    /// The manifest under `digest`, or `None` for one not held.
    fn manifest(&self, digest: &str) -> impl Future<Output = Option<Manifest>> + Send;

    /// The blob under `digest`, or `None` for one not held.
    fn blob(&self, digest: &str) -> impl Future<Output = Option<Self::Blob>> + Send;

    /// The proof that the image is held: `SHA-256(nonce ‖ D)` over the
    /// image `digest` names — with `platform`, the manifest in it
    /// meant, when `digest` names an index — exactly as
    /// [`oci::prove`](crate::shared::containers::oci::prove) defines
    /// `D`; [`Prover`](crate::shared::containers::oci::prove::proof::Prover)
    /// computes it. `None` for an image not held whole. A caller whose
    /// image is in another provider's Store answers from there,
    /// through [`images::hash`](crate::provider::endpoints::images::hash),
    /// and moves no byte of it.
    fn prove(&self, digest: &str, platform: Option<&str>, nonce: &[u8; 32]) -> impl Future<Output = Option<[u8; 32]>> + Send;
}
