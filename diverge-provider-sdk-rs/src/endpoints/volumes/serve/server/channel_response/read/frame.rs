//! What a server's channel response frame carries on a read
//! channel of a serve.

/// The piece's bytes, that there is no file, or an error. See [`fuse::read::response::Frame`](crate::shared::containers::fuse::read::response::Frame).
///
/// An alias rather than a re-export because this module is real: the
/// path says this scope's answer lives here, and it does, rather than
/// naming somewhere else and hoping a reader follows. What the alias
/// points at is right there in the signature.
pub type Frame<'a> = crate::shared::containers::fuse::read::response::Frame<'a>;
