//! What a provider sends back on a watch.
//!
//! A [`filetree`](crate::filetree) stream: one snapshot carrying the
//! whole tree, then one frame per change for as long as the caller
//! watches. Every path in it is relative to the directory the watch
//! named.
//!
//! # One alias, for what rides the channel
//!
//! [`Frame`] and nothing else. It is an alias because a watch's answer
//! IS a filetree stream — defining it again would be two definitions
//! of one thing waiting to disagree, and
//! [`Root::update`](crate::filetree::response::Root::update) would be
//! where they did.
//!
//! What the frame CONTAINS is not aliased.
//! [`Node`](crate::filetree::response::Node) is named through
//! [`filetree::response`](crate::filetree::response), which is where
//! it is defined. So is [`Root`](crate::filetree::response::Root),
//! which is not a response at all — nothing sends one; it is what a
//! caller folds the stream INTO, and a module named for what a
//! provider sends is the wrong place to find it.
//!
//! Mirroring the whole module instead would mean every type added to
//! filetree has to be re-aliased here — a can of worms, and one that
//! grows every time filetree does.

mod frame;

pub use frame::*;
