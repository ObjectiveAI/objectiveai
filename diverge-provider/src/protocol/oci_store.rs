//! What a provider serves of an image for a caller, on the caller's
//! own asks.

use diverge_sdk::shared::Identity;
use std::future::Future;

use bytes::Bytes;
use futures_util::Stream;

use super::image_source::Manifest;
use diverge_sdk::shared::containers::request::Image;

/// The images a provider serves to a caller's pull: what the
/// `images::read` scope answers from, one image per scope.
///
/// The mirror of the caller half's `OciStore`: there a caller answers
/// a provider's asks from what it holds, and here a provider answers
/// a caller's. The three questions are the same — a manifest, a blob,
/// a proof of possession — and so are the answers, down to the empty
/// finish for a digest not served.
///
/// # One image per scope
///
/// A scope names its image as it opens, and [`open`](Self::open)
/// resolves it once: `Ok(None)` is the scope's `NotFound`, [`Err`]
/// its `Error`, and anything else is a [`ServedImage`] — the scope's
/// `Held` — that answers every ask of the scope and is
/// [`close`](ServedImage::close)d, explicitly, when the scope ends —
/// so a provider that brings bytes in to serve them
/// holds them for exactly the scope's life, and decides at its end
/// what to keep.
///
/// # Served FOR the caller
///
/// `open` takes the caller's identity, because what a provider serves
/// one identity is not what it serves another: the identity's own
/// Store, the images it built or pushed through the
/// [`ImageManager`](super::image_manager::ImageManager), held byte for
/// byte as the provider fixed them when ownership was gained; or a
/// registry the provider uses that one of the image's references
/// names and that has the manifest. Another
/// identity's images are never served, however alike the digests; nor
/// is an image the provider merely pulled, whose bytes its runtime
/// recompresses on the way out, so that nothing it could serve hashes
/// to the digest asked for.
///
/// # An error opens, and nothing after
///
/// A store that cannot resolve an image — a registry that could not be
/// asked, or that answered bytes that are not the digest's — fails
/// the open with its [`Error`](Self::Error), which the handler sends
/// as the scope's `Error`: the absence of an answer, never folded into
/// a `NotFound` a caller would read as the image not held. Once the
/// image is open there is no error vocabulary: a served image that
/// cannot read or fetch a part, or prove itself, answers `None` for
/// the ask, the empty finish, the wire's could-not-serve. There is
/// nothing an error could say then that would change what the caller
/// does next.
pub trait OciStore: Send + Sync {
    /// Whatever this provider's resolution fails with: flattened into
    /// the protocol's error by the handler, at the point where a frame
    /// is written. A failure to ANSWER, not a negative answer.
    type Error: Send + 'static;

    /// One image, as one scope serves it.
    type Served: ServedImage + 'static;

    /// The image resolved for the caller; `Ok(None)` where it is not
    /// served to it.
    fn open(&self, client_identity: &Identity, image: &Image) -> impl Future<Output = Result<Option<Self::Served>, Self::Error>> + Send;
}

/// One image a scope serves: its manifests and blobs by digest, and
/// only the ones the image reaches — its manifest; every manifest an
/// index lists; the config and the layers of each image manifest
/// reached. Any other digest is `None`, however it is known. And its
/// proof of possession, over the same bytes.
pub trait ServedImage: Send + Sync {
    /// A blob's pieces, in order. Piece sizes are the store's; the
    /// handler re-splits at [`CHUNK_SIZE`](diverge_sdk::CHUNK_SIZE).
    type Blob: Stream<Item = Bytes> + Send + 'static;

    /// The manifest under `digest`, which the image reaches, or `None`.
    fn manifest(&self, digest: &str) -> impl Future<Output = Option<Manifest>> + Send;

    /// The blob under `digest`, which the image reaches, or `None`.
    fn blob(&self, digest: &str) -> impl Future<Output = Option<Self::Blob>> + Send;

    /// The proof of possession over the image, keyed by `nonce` — see
    /// [`prove`](diverge_sdk::shared::containers::oci::prove), the
    /// computation a provider's own nonce check makes — over the bytes
    /// the scope serves, those of a registry brought in first. `None`
    /// is a `platform` that breaks the rule — required when the
    /// manifest is an index, and one of the manifests it lists; absent
    /// otherwise — and an image whose parts could not be read or
    /// fetched. No proof is ever computed over an index alone.
    fn prove(&self, platform: Option<&str>, nonce: &[u8; 32]) -> impl Future<Output = Option<[u8; 32]>> + Send;

    /// The scope over: whatever the image held for it let go. Called
    /// once, after the last ask is answered or abandoned; nothing is
    /// asked of the image after.
    fn close(&self) -> impl Future<Output = ()> + Send;
}
