//! What a server's response frame carries on a blob channel.

/// One piece of a blob the provider holds for the caller. See [`oci::blob::response::Frame`](crate::shared::containers::oci::blob::response::Frame).
///
/// An alias rather than a re-export because this module is real: the
/// path says this scope's answer lives here, and it does, rather than
/// naming somewhere else and hoping a reader follows. What the alias
/// points at is right there in the signature.
pub type Frame<'a> = crate::shared::containers::oci::blob::response::Frame<'a>;
