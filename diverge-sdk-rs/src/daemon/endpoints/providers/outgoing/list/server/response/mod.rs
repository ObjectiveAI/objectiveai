//! The list response: the outgoing providers, one value each, or a
//! failure.
//!
//! [`Frame`] is what a response frame holds — one value, forbidden, or
//! a failure. [`Outgoing`] is what a value is without a program, and
//! the reference for what a program is run over.

mod frame;
mod outgoing;

pub use frame::*;
pub use outgoing::*;
