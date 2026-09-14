//! Where the content a caller mounts by identity is kept.

use std::future::Future;

use bytes::Bytes;
use futures_util::Stream;

/// The provider's store of mounted content, keyed by identity.
///
/// An
/// [`IdentityMount`](crate::shared::containers::request::IdentityMount)
/// names content by `<size>:<hash>` — a file by the base64url
/// SHA-256 of its bytes, a directory by Go's `h1:` directory hash —
/// and says where it
/// goes; the content itself is somewhere the provider keeps it, and
/// this is that somewhere. A run handler asks [`holds`](Self::holds)
/// for every identity a request names, fetches from the caller only
/// what is not held — a provider may have any of them already, from
/// an earlier run of anyone's — and stores what arrives, all of it
/// BEFORE the deploy, since a
/// [`Deployment`](super::deployment::Deployment) names identities the
/// deployer binds from here and every one must be present by then.
///
/// # The store verifies; this crate does not
///
/// What is stored under an identity is what the caller sent, and a
/// caller can send the wrong bytes. The identity carries the size and
/// the hash — of the bytes for a file, of the manifest for a
/// directory, as the mount's own documentation states — and an
/// implementation checks them before it lets the identity count as
/// held, the way its registry checks a digest. A store that took the
/// caller's word would mount whatever the caller said under a name
/// that promised otherwise. This crate hashes nothing: it does not
/// know where the bytes land, and a hash computed on the way through
/// would be a second one beside the store's.
///
/// So a [`store_file`](Self::store_file) whose bytes do not match is
/// an [`Err`](Self::Error), and so is one whose stream ended short —
/// the caller's connection dying mid-content — which the hash catches
/// without anything here having to say so.
///
/// # Streams, not buffers
///
/// Content is whatever size the caller made it. A store takes it as
/// it arrives and writes it where it goes; nothing upstream holds a
/// whole file to hand it over as one piece.
pub trait ContentStore: Send + Sync {
    /// Why content could not be stored: the bytes did not match the
    /// identity, the disk is full, the stream ended short. The
    /// provider's own; it reaches the caller as the run's
    /// [`Error`](crate::shared::error::Error).
    type Error: Send + 'static;

    /// Whether `identity` is held: present, verified, mountable.
    fn holds(&self, identity: &str) -> impl Future<Output = bool> + Send;

    /// Store a file's bytes under `identity`, from its pieces in
    /// order. Held once this is [`Ok`].
    fn store_file<S>(&self, identity: &str, content: S) -> impl Future<Output = Result<(), Self::Error>> + Send
    where
        S: Stream<Item = Bytes> + Send + 'static;

    /// Store a directory under `identity`, from its files in whatever
    /// order they come: each item a relative path in components and a
    /// piece of that file's bytes, adjacent items with an equal path
    /// one file in order, a new path a new file. Held once this is
    /// [`Ok`].
    fn store_directory<S>(&self, identity: &str, files: S) -> impl Future<Output = Result<(), Self::Error>> + Send
    where
        S: Stream<Item = (Vec<String>, Bytes)> + Send + 'static;
}
