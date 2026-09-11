//! What a provider sends back on a volume creation.
//!
//! One frame, once, and it says the volume exists, or that there was
//! no room for it — see [`Frame`]. [`Creation`] is the two answers a
//! manager can give without failing, and the frame is made from it.

mod creation;
mod frame;

pub use creation::*;
pub use frame::*;
