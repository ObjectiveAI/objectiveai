//! Volume edit request data.
//!
//! [`Frame`] names the volume and the [`Change`] to make to it: its
//! size, its persist mode, or both.

mod change;
mod frame;

pub use change::*;
pub use frame::*;
