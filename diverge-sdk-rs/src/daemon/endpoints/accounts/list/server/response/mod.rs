//! The list response: the accounts, one value each, forbidden, or a
//! failure.
//!
//! [`Frame`] is what a response frame holds — one value, forbidden, or
//! a failure. [`Account`] is what a value is without a program, and the
//! reference for what a program is run over.

mod account;
mod frame;

pub use account::*;
pub use frame::*;
