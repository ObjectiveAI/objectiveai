//! The list response: the outgoing providers, one value each, or a
//! failure.
//!
//! [`Frame`] is what a response frame holds — one [`Outgoing`],
//! forbidden, or a failure. [`Outgoing`] is what one is.

mod frame;
mod outgoing;

pub use frame::*;
pub use outgoing::*;
