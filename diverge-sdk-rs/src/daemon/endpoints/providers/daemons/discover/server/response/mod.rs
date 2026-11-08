//! The discover response: the identities, one value each, or a
//! failure.
//!
//! [`Frame`] is what a response frame holds — one [`Discovered`],
//! forbidden, or a failure. [`Discovered`] is what one is, and [`Via`]
//! one route to it.

mod discovered;
mod frame;
mod via;

pub use discovered::*;
pub use frame::*;
pub use via::*;
