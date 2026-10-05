//! The list response: the judges, one value each, forbidden, or a
//! failure.
//!
//! [`Frame`] is what a response frame holds — one value, forbidden, or
//! a failure. [`Incoming`] is what a value is without a program, and
//! the reference for what a program is run over.

mod frame;
mod incoming;

pub use frame::*;
pub use incoming::*;
