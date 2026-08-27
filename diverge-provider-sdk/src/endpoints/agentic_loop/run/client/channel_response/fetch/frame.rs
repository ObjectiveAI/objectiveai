//! What a client's response frame carries on a fetch channel.

/// One file of the directory being fetched.
///
/// An alias, because what a channel carries here is what
/// [`shared::fetch`](crate::shared::fetch) says it carries. See
/// [`fetch::response::Frame`](crate::shared::fetch::response::Frame)
/// for what it is: one frame per file, the finish saying the
/// directory is whole, and zero frames before it saying the client
/// does not hold the hash.
///
/// An alias rather than a re-export because this module is real. The
/// path says this endpoint's answer to a fetch lives here, and it
/// does, rather than naming somewhere else and hoping a reader
/// follows. What the alias points at is right there in the signature.
pub type Frame = crate::shared::fetch::response::Frame;
