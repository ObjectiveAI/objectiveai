//! What a provider holds of an image for a caller, served to the
//! caller's own asks.

use std::future::Future;

use bytes::Bytes;
use futures_util::Stream;

use super::image_source::Manifest;

/// A store of manifests and blobs indexed by digest, read for one
/// caller: what the `images::oci` scope answers from.
///
/// The mirror of the caller half's `OciStore`: there a caller answers
/// a provider's asks from what it holds, and here a provider answers
/// a caller's. The three questions are the same — whether the image
/// is held, and under what repository path; its manifest; a blob —
/// and so are the answers, down to the empty finish for a digest not
/// held.
///
/// # Held FOR the caller
///
/// Every method takes the caller's identity, because what a provider
/// holds for one identity is not what it holds for another: the
/// identity's own images, built on this provider through the
/// [`ImageManager`](super::image_manager::ImageManager), and the
/// images the provider holds on its own account, which no client
/// built — the same two sources an
/// [`ImageChecker`](super::image_checker::ImageChecker) answers as
/// `client` and `server`. Another identity's images are not held,
/// however alike the digests, and are never served.
///
/// # No error vocabulary
///
/// A store that cannot read what it holds answers as if it did not
/// hold it: `None`, which the handler sends as the empty finish, the
/// wire's could-not-serve. There is nothing an error could say that
/// would change what the caller does next.
pub trait OciStore: Send + Sync {
    /// Whether the image under `digest` is held for the caller, and
    /// the repository path it is held under, which a registry serving
    /// the caller would serve it as; `None` for one not held.
    fn holds(&self, client_identity: &str, digest: &str) -> impl Future<Output = Option<String>> + Send;

    /// The manifest under `digest`, held for the caller, or `None`.
    fn manifest(&self, client_identity: &str, digest: &str) -> impl Future<Output = Option<Manifest>> + Send;

    /// A blob's pieces, in order. Piece sizes are the store's; the
    /// handler re-splits at [`CHUNK_SIZE`](diverge_sdk::CHUNK_SIZE).
    type Blob: Stream<Item = Bytes> + Send + 'static;

    /// The blob under `digest`, held for the caller, or `None`.
    fn blob(&self, client_identity: &str, digest: &str) -> impl Future<Output = Option<Self::Blob>> + Send;
}
