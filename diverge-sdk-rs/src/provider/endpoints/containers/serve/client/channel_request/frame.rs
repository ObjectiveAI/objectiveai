//! What a client's channel request frame carries on a container
//! serve scope.

/// One of the nine asks, the stop, or the tree: the
/// [`volumes::serve`](crate::provider::endpoints::volumes::serve) scope's
/// channel request, byte for byte — `0`–`8` the mount's asks, `9`
/// the stop, `10` the tree — answered here from a container rather
/// than a volume. See [`Frame`](crate::provider::endpoints::volumes::serve::client::channel_request::Frame)
/// for every tag.
///
/// An alias rather than a re-export because this module is real: the
/// path says this scope's channel request lives here, and it does,
/// rather than naming somewhere else and hoping a reader follows.
/// What the alias points at is right there in the signature. One
/// type rather than two because the whole point is that a bridge
/// forwards bytes: an ask a mount makes goes to a volume or to a
/// container in the same frame, and nothing is re-encoded on the way.
pub type Frame<'a> = crate::provider::endpoints::volumes::serve::client::channel_request::Frame<'a>;

/// A container serve channel request that could not be read: the
/// volume serve's own failures. See
/// [`FrameError`](crate::provider::endpoints::volumes::serve::client::channel_request::FrameError).
pub type FrameError = crate::provider::endpoints::volumes::serve::client::channel_request::FrameError;
