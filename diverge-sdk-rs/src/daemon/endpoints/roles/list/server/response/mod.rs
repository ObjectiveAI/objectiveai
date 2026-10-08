//! The list response: a role added, changed or removed, the word
//! that the list is whole, forbidden, or a failure.
//!
//! [`Frame`] is what a response frame holds — one [`Role`] as it comes
//! to be listed, changes, or goes, the word, forbidden, or a failure.
//! [`Role`] is what one role is.

mod frame;
mod role;

pub use frame::*;
pub use role::*;
