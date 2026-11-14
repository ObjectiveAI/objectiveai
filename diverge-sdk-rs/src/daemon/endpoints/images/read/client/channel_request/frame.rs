//! What a client's channel request frame carries on the daemon's oci
//! scope.

/// The provider's oci channel requests, the same tags and the same
/// bodies: `0` a manifest, `1` a blob, `2` the proof, `3` the cancel.
/// See [`images::read::client::channel_request::Frame`](crate::provider::endpoints::images::read::client::channel_request::Frame).
///
/// An alias rather than a re-export because this module is real: the
/// path says this scope's channel requests live here, and they do,
/// rather than naming somewhere else and hoping a reader follows. What
/// the alias points at is right there in the signature.
pub type Frame = crate::provider::endpoints::images::read::client::channel_request::Frame;

/// A channel request that could not be read: the provider scope's own.
pub type FrameError = crate::provider::endpoints::images::read::client::channel_request::FrameError;
