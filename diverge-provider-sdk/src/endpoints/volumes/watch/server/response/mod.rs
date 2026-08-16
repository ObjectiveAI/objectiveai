//! What a provider sends back on a watch.
//!
//! A [`filetree`](crate::shared::filetree) stream: one snapshot carrying the
//! whole tree, then one frame per change for as long as the caller
//! watches. Every path in it is relative to the volume the watch
//! named.
//!
//! # Not an alias
//!
//! [`Frame`] wraps
//! [`filetree::response::Frame`](crate::shared::filetree::response::Frame)
//! rather than naming it, because a watch can also fail and a filetree
//! cannot — what a change to a tree looks like is shared, and what can
//! go wrong watching one belongs to this endpoint.
//!
//! What the frame CONTAINS is not redefined.
//! [`Node`](crate::shared::filetree::response::Node) is named through
//! [`filetree::response`](crate::shared::filetree::response), which is where
//! it is defined. So is [`Root`](crate::shared::filetree::response::Root),
//! which is not a response at all — nothing sends one; it is what a
//! caller folds the stream INTO, and a module named for what a
//! provider sends is the wrong place to find it.
//!
//! Mirroring the whole module instead would mean every type added to
//! filetree has to be re-aliased here — a can of worms, and one that
//! grows every time filetree does.

mod frame;

pub use frame::*;
