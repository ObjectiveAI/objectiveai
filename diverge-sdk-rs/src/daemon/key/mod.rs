//! What an agent or a tool is, once and for all, with its name
//! beside: how one list item points at another.
//!
//! A name is optional and may be given again once its holder is
//! deleted, so nothing that has to keep pointing at an agent or a
//! tool points by name. An [`Agent`] is its template and its index;
//! a [`Tool`] is its [`Origin`] — the template it was made from, or
//! the provider and container id it joined — and its index; each
//! carries its name beside, when it has one, for a reader. The
//! agents list names a tool's attachments this way and the tools
//! list an agent's, and a [`creator`](crate::daemon::creator) names
//! a maker the same way, less the origin a creator never needs.

mod agent;
mod origin;
mod tool;

pub use agent::*;
pub use origin::*;
pub use tool::*;
