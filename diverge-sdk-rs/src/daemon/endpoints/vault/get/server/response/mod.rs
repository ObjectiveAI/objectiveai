//! The get response: the key with its values, no such key, forbidden,
//! or a failure.
//!
//! [`Frame`] is what a response frame holds; [`Entry`] is what the key
//! comes back as — what a list reports of it, with every value
//! beside, each a [`Slot`] with its number and its lock.

mod entry;
mod frame;
mod slot;

pub use entry::*;
pub use frame::*;
pub use slot::*;
