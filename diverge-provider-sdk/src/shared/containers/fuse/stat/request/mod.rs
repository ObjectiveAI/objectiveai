//! The ask: the mount, and the entry's path in it — a
//! [`Target`](super::super::Target), empty for a file mount.

/// Describe this entry.
pub type Request<'a> = super::super::Target<'a>;
