//! Filetree response data.
//!
//! A filetree response is a **stream**: one [`Frame::Snapshot`]
//! carrying the whole tree, then one delta per change — inserted,
//! modified, or removed — for as long as the caller watches. There
//! is no polling: the snapshot establishes the tree, every later
//! frame names one node, and the tree is sent whole again only when
//! the source lost track of it, as a second snapshot that replaces
//! the first.
//!
//! The rules governing that sequence — snapshot first, deltas only
//! after it, a snapshot again only for a source that lost track —
//! are ordering properties over a stream, which no schema can
//! express. They belong to the prose specification. What this module
//! defines is the vocabulary: what a node is, and what a frame is.

mod frame;
mod node;
mod root;

pub use frame::*;
pub use node::*;
pub use root::*;
