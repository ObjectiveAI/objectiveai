//! The daemon's own tools, as a template hands them to the agents
//! and the tools made from it.
//!
//! Besides the tool containers attached to an agent, an agent — or a
//! tool container the daemon runs — may call tools the daemon itself
//! answers: over the caller's agents, over the templates they are
//! made from, and over their tags. Which of them a container has is
//! its template's: the [`DaemonTools`] a
//! [template](crate::daemon::template) carries names each tool the
//! containers made from it hold, and a container holds exactly the
//! tools named and no other. A template that carries no
//! `daemon_tools` makes containers that hold none.
//!
//! Each tool's shape is here, one file each, and what each tool does
//! and how far it reaches is not yet stated: a member whose shape is
//! an empty object says only that the agent has the tool.

mod agents;
mod agents_create;
mod agents_delete;
mod agents_edit;
mod daemon_tools;
mod tags;
mod templates_delete;

pub use agents::*;
pub use agents_create::*;
pub use agents_delete::*;
pub use agents_edit::*;
pub use daemon_tools::*;
pub use tags::*;
pub use templates_delete::*;
