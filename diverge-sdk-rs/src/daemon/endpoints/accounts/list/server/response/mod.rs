//! The list response: the accounts, one each, forbidden, or a failure.
//!
//! [`Frame`] is what a response frame holds — one [`Account`],
//! forbidden, or a failure. [`Account`] is what one is.

mod account;
mod frame;

pub use account::*;
pub use frame::*;
