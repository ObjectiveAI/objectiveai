//! The list response: the providers, one value each, or a
//! failure.
//!
//! [`Frame`] is what a response frame holds — one [`Provider`],
//! forbidden, or a failure. [`Provider`] is what one is.

mod frame;
mod provider;

pub use frame::*;
pub use provider::*;
