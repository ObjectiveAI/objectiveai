//! What a client's response frame carries on a fuse setattr channel.

/// Ok, or why not — the attributes are set, or they are not. See [`fuse::setattr::response::Frame`](crate::shared::containers::fuse::setattr::response::Frame).
///
/// An alias rather than a re-export because this module is real: the
/// path says this scope's answer lives here, and it does, rather than
/// naming somewhere else and hoping a reader follows. What the alias
/// points at is right there in the signature.
pub type Frame<'a> = crate::shared::containers::fuse::setattr::response::Frame<'a>;
