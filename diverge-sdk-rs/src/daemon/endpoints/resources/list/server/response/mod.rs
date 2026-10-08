//! The list response: a resource added, changed or removed, the word
//! that the list is whole, forbidden, or a failure.
//!
//! [`Frame`] is what a response frame holds — one [`Listed`] as it
//! comes to be listed, changes, or goes, the word, forbidden, or a
//! failure. [`Listed`] is what one resource is.

mod frame;
mod listed;

pub use frame::*;
pub use listed::*;
