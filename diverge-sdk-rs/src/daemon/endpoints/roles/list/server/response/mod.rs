//! The list response: the roles, one each, forbidden, or a failure.
//!
//! [`Frame`] is what a response frame holds — one [`Role`], forbidden,
//! or a failure. [`Role`] is what one is.

mod frame;
mod role;

pub use frame::*;
pub use role::*;
