//! The list response: the volumes, one each, forbidden, or a failure.
//!
//! [`Frame`] is what a response frame holds — one [`Volume`],
//! forbidden, or a failure. [`Volume`] is what one is.

mod frame;
mod volume;

pub use frame::*;
pub use volume::*;
