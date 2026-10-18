//! What a provider sends back on an image deletion.
//!
//! One frame, once, and it says the image is gone, or that a running
//! container runs it — see [`Frame`]. [`Deletion`] is the two answers
//! a manager can give without failing, and the frame is made from it.

mod deletion;
mod frame;

pub use deletion::*;
pub use frame::*;
