//! The list response: the judges, one value each, or a failure.
//!
//! [`Frame`] is what a response frame holds — one value, or a failure. [`Incoming`] is what a value is without a program, and the reference for what a program is run over.

mod frame;
mod incoming;

pub use frame::*;
pub use incoming::*;
