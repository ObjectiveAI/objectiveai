//! The container's filesystem, from the server's side: watched, read,
//! written.
//!
//! Three scopes, each the last hop of a channel the caller opens on
//! the provider: [`tree`] is the container's tree as a stream,
//! [`read`] one file's bytes, [`write`](mod@write) one file put in
//! place. Their shapes are
//! [`shared::containers`](crate::shared::containers)'s and
//! [`shared::filetree`](crate::shared::filetree)'s where they exist
//! there; what is this wire's own is what the server names to open
//! each.

pub mod read;
pub mod tree;
pub mod write;
