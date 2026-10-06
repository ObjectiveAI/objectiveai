//! The filetree response: the snapshot and the changes, one frame each,
//! no such agent, forbidden, or a failure.
//!
//! [`Frame`] is what a response frame holds. A change is the provider
//! protocol's own [`filetree`](crate::shared::filetree) frame, carried
//! as it is.

mod frame;

pub use frame::*;
