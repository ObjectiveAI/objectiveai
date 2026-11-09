//! The list response: a tool added, changed or removed, the word
//! that the list is whole, forbidden, or a failure.
//!
//! [`Frame`] is what a response frame holds — one [`Tool`] as it comes
//! to be listed, changes, or goes, the word, forbidden, or a failure. [`Tool`] is what one is: one tool as the daemon holds
//! it — its name, the template it was made from and its index, who
//! made it — its
//! [`creator`](crate::daemon::creator) — whether its container runs
//! and where, the agents it is attached to, and its tags.

mod frame;
mod tool;

pub use frame::*;
pub use tool::*;
