//! The list response: a connected tool added, changed or removed, the
//! word that the list is whole, forbidden, or a failure.
//!
//! [`Frame`] is what a response frame holds — one
//! [`ConnectedStandaloneTool`] as it comes to be listed, changes, or
//! goes, the word, forbidden, or a failure.
//! [`ConnectedStandaloneTool`] is what one is: one connected tool as
//! the daemon holds it — its name, the daemon it is on, its route and
//! the tool there, its index, who made it — its
//! [`creator`](crate::daemon::creator) — whether its connect is held,
//! the agents it is attached to, and its tags. [`Key`] is what a
//! listing tells one from another by.

mod frame;
mod key;
mod tool;

pub use frame::*;
pub use key::*;
pub use tool::*;
