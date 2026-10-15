//! The list response: an account added, changed or removed, the
//! word that the list is whole, forbidden, or a failure.
//!
//! [`Frame`] is what a response frame holds — one [`Account`] as it
//! comes to be listed, changes, or goes, the word, forbidden, or a
//! failure. [`Account`] is what one account is.

mod account;
mod frame;

pub use account::*;
pub use frame::*;
