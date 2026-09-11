//! The ask: the mount, and the new directory's path in it — a
//! [`Target`](super::super::Target), never empty, its parent an
//! existing directory.

/// Make this directory.
pub type Request<'a> = super::super::Target<'a>;
