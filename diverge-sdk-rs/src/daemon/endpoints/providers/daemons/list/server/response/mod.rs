//! The list response: the identities, one value each, or a
//! failure.
//!
//! [`Frame`] is what a response frame holds — one [`Listed`],
//! forbidden, or a failure. [`Listed`] is what one is, and [`Via`]
//! one route to it.

mod listed;
mod frame;
mod via;

pub use listed::*;
pub use frame::*;
pub use via::*;
