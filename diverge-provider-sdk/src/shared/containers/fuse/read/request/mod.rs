//! The ask: the mount, and the file's path in it — a
//! [`Target`](super::super::Target), empty for a file mount.

/// Read this file, whole.
pub type Request<'a> = super::super::Target<'a>;
