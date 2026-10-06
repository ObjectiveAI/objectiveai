//! The list response: the volumes, one value each, forbidden, or a
//! failure.
//!
//! [`Frame`] is what a response frame holds — one value, forbidden, or
//! a failure. [`Volume`] is what a value is without a program, and the
//! reference for what a program is run over.

mod frame;
mod volume;

pub use frame::*;
pub use volume::*;
