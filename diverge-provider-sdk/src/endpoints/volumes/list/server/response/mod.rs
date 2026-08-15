//! Volume listing response data.
//!
//! [`Frame`] is what comes back on channel `0`, and [`Volume`] is what
//! it is made of.

mod frame;
mod volume;

pub use frame::*;
pub use volume::*;
