//! What an agent or a tool is, once and for all, with its name
//! beside: how one list item points at another.
//!
//! A name is optional and may be given again once its holder is
//! deleted, so nothing that has to keep pointing at an agent or a
//! tool points by name. An [`Agent`] is its template and its index;
//! a [`Tool`] is a standalone tool's template and its index, a
//! connected tool's daemon, its tool there and its index, or a
//! dependency tool's template and its [`Owner`] — the agent, the agent
//! template, or everyone; each carries its name beside, when it has
//! one, for a reader. The agents list names a tool's attachments this
//! way and the tools lists an agent's, and a
//! [`creator`](crate::daemon::creator) names a maker the same way.

mod agent;
mod owner;
mod tool;

pub use agent::*;
pub use owner::*;
pub use tool::*;
