//! The list response: the credentials, one each, forbidden, or a
//! failure.
//!
//! [`Frame`] is what a response frame holds — one [`Incoming`],
//! forbidden, or a failure. [`Incoming`] is what one is.

mod frame;
mod incoming;

pub use frame::*;
pub use incoming::*;
