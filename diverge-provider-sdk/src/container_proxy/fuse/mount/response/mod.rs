//! The answer: the mount made, or why not — the fuse family's ack.

/// `0` ok, once the mount is serving; `1` and a message, the mount
/// not made.
pub type Frame<'a> = crate::shared::containers::fuse::ack::Frame<'a>;
