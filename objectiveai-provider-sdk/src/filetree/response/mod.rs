//! Filetree response data.
//!
//! A filetree response is a **stream**: one [`Frame::Snapshot`]
//! carrying the whole tree, then one delta per change — inserted,
//! modified, moved, or removed — for as long as the caller watches.
//! There is no polling and no second full send: the snapshot
//! establishes the tree, and every later frame names one node.
//!
//! The rules governing that sequence — snapshot first, exactly once,
//! deltas only after it — are ordering properties over a stream, which
//! no schema can express. They belong to the prose specification. What
//! this module defines is the vocabulary: what a node is, and what an
//! a frame is.

mod frame;
mod node;
mod root;

pub use frame::*;
pub use node::*;
pub use root::*;
