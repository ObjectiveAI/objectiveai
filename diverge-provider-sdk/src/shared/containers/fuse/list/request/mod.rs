//! The ask: the mount, and the directory's path in it — a
//! [`Target`](super::super::Target), empty for the root.

/// List this directory.
pub type Request<'a> = super::super::Target<'a>;
