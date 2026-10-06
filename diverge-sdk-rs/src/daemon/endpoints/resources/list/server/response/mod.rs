//! The list response: the resources, one value each, forbidden, or a
//! failure.
//!
//! [`Frame`] is what a response frame holds — one value, forbidden, or
//! a failure. [`Listed`] is what a value is without a program, and the
//! reference for what a program is run over.

mod frame;
mod listed;

pub use frame::*;
pub use listed::*;
