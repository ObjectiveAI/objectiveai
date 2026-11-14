//! What a client's channel request frame carries on the daemon's FUSE
//! serve scope.

/// The provider's FUSE serve channel requests, the same tags and the
/// same bodies: `0` to `8` the nine asks, `9` the cancel, `10` the
/// file tree. See
/// [`volumes::fuse_serve::client::channel_request::Frame`](crate::provider::endpoints::volumes::fuse_serve::client::channel_request::Frame).
///
/// An alias rather than a re-export because this module is real: the
/// path says this scope's channel requests live here, and they do,
/// rather than naming somewhere else and hoping a reader follows. What
/// the alias points at is right there in the signature.
pub type Frame<'a> = crate::provider::endpoints::volumes::fuse_serve::client::channel_request::Frame<'a>;

/// A channel request that could not be read: the provider scope's own.
pub type FrameError = crate::provider::endpoints::volumes::fuse_serve::client::channel_request::FrameError;
