//! The mounted content a caller holds, by identity.

use std::future::Future;

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
    /// A file's pieces, in order. Piece sizes are the store's; the
    /// executor re-splits at [`CHUNK_SIZE`](crate::CHUNK_SIZE).
    type File: Stream<Item = Bytes> + Send + 'static;
    /// A directory's files in any order — each a relative path in
    /// components and the file's bytes, whole or in adjacent pieces.
    /// The executor sends each piece under its path, split at
    /// [`CHUNK_SIZE`](crate::CHUNK_SIZE).
    type Directory: Stream<Item = (Vec<String>, Bytes)> + Send + 'static;

    /// The file under `identity`, or `None`.
    fn file(&self, identity: &str) -> impl Future<Output = Option<Self::File>> + Send;

    /// The directory under `identity`, or `None`.
    fn directory(&self, identity: &str) -> impl Future<Output = Option<Self::Directory>> + Send;
}
