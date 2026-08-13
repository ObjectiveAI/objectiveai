//! What a provider sends back on a watch.
//!
//! A [`filetree`](crate::filetree) stream: one [`Frame::Snapshot`]
//! carrying the whole tree, then one frame per change for as long as
//! the caller watches. Every path in it is relative to the directory
//! the watch named.
//!
//! # Aliases, all three
//!
//! Nothing is defined here. A watch's answer IS a filetree stream, so
//! describing it again would be two definitions of one thing waiting
//! to disagree — and the fold in [`Root::update`] would be the thing
//! that disagreed.
//!
//! Aliases rather than a re-export of
//! [`filetree::response`](crate::filetree::response), because this
//! module is real. A reader
//! looking for what a watch answers with finds it here, under the path
//! that says so, and each alias names its own target in its own
//! signature rather than redirecting somewhere else.

mod frame;
mod node;
mod root;

pub use frame::*;
pub use node::*;
pub use root::*;
