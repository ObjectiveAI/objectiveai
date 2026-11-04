//! The discover response: the identities, one value each, or a
//! failure.
//!
//! [`Frame`] is what a response frame holds — one [`Discovered`],
//! forbidden, or a failure. [`Discovered`] is what one is.

mod discovered;
mod frame;

pub use discovered::*;
pub use frame::*;
