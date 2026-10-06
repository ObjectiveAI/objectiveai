//! The list response: the resources, one each, forbidden, or a failure.
//!
//! [`Frame`] is what a response frame holds — one [`Listed`],
//! forbidden, or a failure. [`Listed`] is what one is.

mod frame;
mod listed;

pub use frame::*;
pub use listed::*;
