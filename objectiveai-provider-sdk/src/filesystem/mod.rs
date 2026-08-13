//! Filesystem — what a provider will let a caller look at.
//!
//! [`list`] is the only thing here so far: which directories exist to
//! be watched. Watching one is [`filetree`](crate::filetree)'s job,
//! and this is how a caller finds out what it may name.

pub mod list;
