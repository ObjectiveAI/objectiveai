//! The list response: a key added, changed or removed, the word that
//! the list is whole, forbidden, or a failure.
//!
//! [`Frame`] is what a response frame holds — one [`Key`] as it comes
//! to be listed, changes, or goes, the word, forbidden, or a failure.
//! [`Key`] is what one key is, its value apart.

mod frame;
mod key;

pub use frame::*;
pub use key::*;
