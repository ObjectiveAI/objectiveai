//! What a provider sends back on a volume deletion.
//!
//! One frame, once, and it says the volume is gone, or that it is
//! mounted and stays — see [`Frame`]. [`Deletion`] is the two answers
//! a manager can give without failing, and the frame is made from it.

mod deletion;
mod frame;

pub use deletion::*;
pub use frame::*;
