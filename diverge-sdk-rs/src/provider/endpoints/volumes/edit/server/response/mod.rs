//! What a provider sends back on a volume edit.
//!
//! One frame, once, and it says the size is what was asked for, or
//! which of two reasons it is not — see [`Frame`]. [`Edit`] is the
//! three answers a manager can give without failing, and the frame is
//! made from it.

mod edit;
mod frame;

pub use edit::*;
pub use frame::*;
