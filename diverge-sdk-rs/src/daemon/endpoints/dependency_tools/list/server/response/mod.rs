//! The list response: a dependency tool added, changed or removed,
//! the word that the list is whole, forbidden, or a failure.
//!
//! [`Frame`] is what a response frame holds — one [`Listed`] as it
//! comes to be listed, changes, or goes, the word, forbidden, or a
//! failure. [`Listed`] is what one tool is: one dependency tool as the
//! daemon holds it — its template and its owner, when it was made, who
//! made it — its [`creator`](crate::daemon::creator) — its tags, the
//! template itself, the agents that depend on it, each an
//! [`Attached`], and its [`Run`], if any.

mod attached;
mod frame;
mod listed;
mod run;

pub use attached::*;
pub use frame::*;
pub use listed::*;
pub use run::*;
