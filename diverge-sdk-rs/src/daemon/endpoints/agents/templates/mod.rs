//! Agent templates: what an agent is made from, held by its hash.
//!
//! The shape and the id are the daemon's one
//! [`template`](crate::daemon::endpoints::template), typed `"agent"`
//! here: everything about an agent that is not its name, not the
//! provider it runs on and not its own mounts. A caller makes one
//! with [`create`] and names it afterwards by its id, so the same
//! template made twice is one template; an [`agent`](super) is
//! created from a template by that id, with a name, a provider and
//! mounts of its own; [`list`] names every template the caller has;
//! [`delete`] removes one no agent was made from.

mod agent_type;

pub use agent_type::*;

pub mod create;
pub mod delete;
pub mod list;
