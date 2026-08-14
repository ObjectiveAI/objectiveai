//! What a provider sends back on a creation.
//!
//! A [`filetree`](crate::filetree) stream over the container's own
//! filesystem: one snapshot, then one frame per change, for as long as
//! the scope lives. The same thing
//! [`filesystem::watch`](crate::filesystem::watch) answers with, over
//! a different tree.
//!
//! Which is why creating and watching are not two asks. A container's
//! filesystem is the observable part of it running, so the scope that
//! made the container is the scope that reports on it, and there is
//! nothing to correlate afterwards.
//!
//! Every path is relative to the container's root. What a provider
//! puts in the tree is its own to decide, and mounted directories are
//! the case worth knowing about: a mount is somebody else's filesystem
//! reached across a boundary that carries no change notifications, so
//! a provider that included one would be promising updates it cannot
//! deliver.
//!
//! # One alias, for what rides the channel
//!
//! [`Frame`] and nothing else, for the reason
//! [`watch`](crate::filesystem::watch::server::response) gives: a
//! filetree stream is a filetree stream, and what a frame CONTAINS is
//! named through [`filetree::response`](crate::filetree::response),
//! where it is defined.

mod frame;

pub use frame::*;
