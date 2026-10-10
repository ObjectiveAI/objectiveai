//! The list response: a tool added, changed or removed, the word
//! that the list is whole, forbidden, or a failure.
//!
//! [`Frame`] is what a response frame holds — one [`Tool`] as it comes
//! to be listed, changes, or goes, the word, forbidden, or a failure. [`Tool`] is what one is: one tool as the daemon holds
//! it — its name, where it comes from — its [`Origin`], created from a
//! template, connected to somebody else's container, or deployed for
//! an agent as a dependency — who made it — its
//! [`creator`](crate::daemon::creator) — whether its container runs,
//! the agents it is attached to, and its tags.

mod frame;
mod origin;
mod tool;

pub use frame::*;
pub use origin::*;
pub use tool::*;
