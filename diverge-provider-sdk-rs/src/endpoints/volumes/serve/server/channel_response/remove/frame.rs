//! What a server's channel response frame carries on a remove
//! channel of a serve.

/// Ok, ephemeral, or why not. See [`fuse::remove::response::Frame`](crate::shared::containers::fuse::remove::response::Frame).
///
/// An alias rather than a re-export because this module is real: the
/// path says this scope's answer lives here, and it does, rather than
/// naming somewhere else and hoping a reader follows. What the alias
/// points at is right there in the signature.
pub type Frame<'a> = crate::shared::containers::fuse::remove::response::Frame<'a>;
