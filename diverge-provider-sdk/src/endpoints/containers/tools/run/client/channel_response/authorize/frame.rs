//! What a client's response frame carries on an authorize channel.

/// Yes or no. See [`authorize::response::Frame`](crate::shared::containers::authorize::response::Frame).
///
/// An alias rather than a re-export because this module is real: the
/// path says this scope's answer lives here, and it does, rather than
/// naming somewhere else and hoping a reader follows. What the alias
/// points at is right there in the signature.
pub type Frame = crate::shared::containers::authorize::response::Frame;
