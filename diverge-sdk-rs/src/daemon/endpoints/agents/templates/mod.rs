//! Agent templates: what an agent is made from, held by its hash.
//!
//! The shape and the id are the daemon's one
//! [`template`](crate::daemon::template), typed `"agent"`
//! here: everything about an agent that is not its name, not the
//! provider it runs on, not its own mounts and not the daemon's own
//! [tools](crate::daemon::daemon_tools) it holds. A caller makes one
//! with [`create`] and names it afterwards by its id, so the same
//! template made twice is one template; an [`agent`](super) is
//! created from a template by that id, with a name, a provider and
//! mounts of its own; [`list`] lists them, narrowed,
//! with their tags; [`delete`] removes one no agent was made from;
//! [`tag`] and [`untag`] change a template's tags, which are the
//! caller's and not in the template's hash.

mod agent_type;

pub use agent_type::*;

pub mod create;
pub mod delete;
pub mod list;
pub mod tag;
pub mod untag;
