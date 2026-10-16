//! The get response: the key with its value, no such key, forbidden,
//! or a failure.
//!
//! [`Frame`] is what a response frame holds; [`Entry`] is what the key
//! comes back as — what a list reports of it, with the value beside.

mod frame;
mod entry;

pub use frame::*;
pub use entry::*;
