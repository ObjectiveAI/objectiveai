//! What the proxy sends back on a tree.
//!
//! A [`filetree`](crate::shared::filetree) stream: one snapshot
//! carrying the whole tree, then one frame per change for as long as
//! the server watches. Every path in it is from the container's root.
//!
//! # Not an alias
//!
//! [`Frame`] wraps
//! [`filetree::response::Frame`](crate::shared::filetree::response::Frame)
//! rather than naming it, because a watch can also fail and a filetree
//! cannot — what a change to a tree looks like is shared, and what can
//! go wrong watching one belongs to this scope.

mod frame;

pub use frame::*;
