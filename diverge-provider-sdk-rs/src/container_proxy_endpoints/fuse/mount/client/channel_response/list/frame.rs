//! What a client's response frame carries on a list channel.

/// The directory's entries, that there is no such directory, or an error. See [`fuse::list::response::Frame`](crate::shared::containers::fuse::list::response::Frame).
///
/// An alias rather than a re-export because this module is real: the
/// path says this scope's answer lives here, and it does, rather than
/// naming somewhere else and hoping a reader follows. What the alias
/// points at is right there in the signature.
pub type Frame<'a> = crate::shared::containers::fuse::list::response::Frame<'a>;
