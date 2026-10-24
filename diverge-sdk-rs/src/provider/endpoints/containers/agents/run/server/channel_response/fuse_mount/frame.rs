//! What a server's response frame carries for a FUSE mount made while
//! the container runs.

/// `Mounted` once the mount is serving; `Overlaps`, `Taken` or
/// `Unsupported` for a request the provider refuses on its own
/// account, without asking the proxy; `Error` for one the proxy
/// refused, in its words — exactly one, then the finish, and a
/// `Mounted` before any ask of the id. See
/// [`fuse_mount::response::Frame`](crate::shared::containers::fuse_mount::response::Frame).
///
/// An alias rather than a re-export because this module is real: the
/// path says this channel's answer lives here, and it does, rather
/// than naming somewhere else and hoping a reader follows. What the
/// alias points at is right there in the signature.
pub type Frame = crate::shared::containers::fuse_mount::response::Frame;
