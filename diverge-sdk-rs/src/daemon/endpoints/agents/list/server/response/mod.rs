//! The list response: an agent added, changed or removed, the word
//! that the list is whole, forbidden, or a failure.
//!
//! [`Frame`] is what a response frame holds — one [`Agent`] as it
//! comes to be listed, changes, or goes, the word, forbidden, or a
//! failure. [`Agent`] is what one is: one agent as the daemon
//! holds it — its name, its template and its number among the agents
//! made from it, who made it — its [`creator`](crate::daemon::creator)
//! — its activity, its provider, its log's length, the tools attached
//! to it, and its tags.

mod agent;
mod frame;

pub use agent::*;
pub use frame::*;
