//! The mounted content a caller holds, by identity.

use std::future::Future;
use std::pin::Pin;

use bytes::Bytes;
use futures_util::Stream;

/// A store of files and directories indexed by identity — what a
/// caller names in its
/// [`identity_file_mounts`](crate::shared::containers::request::Container::identity_file_mounts)
/// and
/// [`identity_directory_mounts`](crate::shared::containers::request::Container::identity_directory_mounts),
/// and what a provider fetches from it when its own store lacks one.
///
/// An identity is `<size>:<base64url sha256>` — of the bytes for a
/// file, of the manifest for a directory; see
/// [`IdentityMount`](crate::shared::containers::request::IdentityMount).
/// The store indexes however it likes and answers to the identity.
/// `None` is absence, sent as the empty finish — the wire's deliberate
/// "could not serve"; there is no error vocabulary on these exchanges.
pub trait IdentityStore: Send + Sync {
    /// The file under `identity`, as its pieces in order, or `None`.
    /// Piece sizes are the store's; the executor re-splits at
    /// [`CHUNK_SIZE`](crate::CHUNK_SIZE).
    fn file(
        &self,
        identity: &str,
    ) -> impl Future<Output = Option<Pin<Box<dyn Stream<Item = Bytes> + Send + 'static>>>> + Send;

    /// The directory under `identity`, as its files in any order —
    /// each a relative path in components and the file's bytes, whole
    /// or in adjacent pieces — or `None`. The executor sends each
    /// piece under its path, split at [`CHUNK_SIZE`](crate::CHUNK_SIZE).
    fn directory(
        &self,
        identity: &str,
    ) -> impl Future<
        Output = Option<Pin<Box<dyn Stream<Item = (Vec<String>, Bytes)> + Send + 'static>>>,
    > + Send;
}
