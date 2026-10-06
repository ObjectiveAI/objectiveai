//! The filetree response: the tree, no such resource or path, a file at
//! the path, forbidden, or a failure.
//!
//! [`Frame`] is what a response frame holds. The tree is the provider
//! protocol's own [`Node`](crate::shared::filetree::response::Node)s,
//! carried as a `volumes::filetree` carries them.

mod frame;

pub use frame::*;
