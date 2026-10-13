//! The list response: a dependency template added, changed or
//! removed, the word that the list is whole, forbidden, or a failure.
//!
//! [`Frame`] is what a response frame holds — one [`Listed`] as it
//! comes to be listed, changes, or goes, the word, forbidden, or a
//! failure. [`Listed`] is what one template is: one dependency
//! template as the daemon holds it — its id, when it was made, who
//! made it — its [`creator`](crate::daemon::creator) — its tags, the
//! template itself, the agents that depend on it, and the tools
//! running from it, each a [`Run`].

mod frame;
mod listed;
mod run;

pub use frame::*;
pub use listed::*;
pub use run::*;
