//! What a server's response frame carries on a dequeue channel.

/// Whether the queue held anything. See [`dequeue::response::Frame`](crate::shared::containers::dequeue::response::Frame).
///
/// An alias rather than a re-export because this module is real: the
/// path says this scope's answer lives here, and it does, rather than
/// naming somewhere else and hoping a reader follows. What the alias
/// points at is right there in the signature.
pub type Frame = crate::shared::containers::dequeue::response::Frame;
