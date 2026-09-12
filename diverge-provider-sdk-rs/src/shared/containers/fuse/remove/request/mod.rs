//! The ask: the mount, and the entry's path in it — a
//! [`Target`](super::super::Target), never empty: the root cannot be
//! removed.

/// Remove this file, or this empty directory.
pub type Request<'a> = super::super::Target<'a>;
