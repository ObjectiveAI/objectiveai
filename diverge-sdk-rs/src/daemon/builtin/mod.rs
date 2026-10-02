//! The daemon's own tools, as an agent template hands them to the
//! agents made from it.
//!
//! Besides the tool containers attached to it, an agent may call
//! tools the daemon itself answers: over the caller's agents, over
//! the templates they are made from, and over their tags. Which of
//! them an agent has is its template's: the [`Builtin`] an
//! [agent template](crate::daemon::endpoints::agents::templates)
//! carries names each tool the agent holds, and an agent holds
//! exactly the tools named and no other. A template that carries no
//! `builtin` makes agents that hold none.
//!
//! Each tool's shape is here, one file each, and what each tool does
//! and how far it reaches is not yet stated: a member whose shape is
//! an empty object says only that the agent has the tool.

mod agents;
mod agents_create;
mod agents_delete;
mod agents_edit;
mod builtin;
mod tags;
mod templates_delete;

pub use agents::*;
pub use agents_create::*;
pub use agents_delete::*;
pub use agents_edit::*;
pub use builtin::*;
pub use tags::*;
pub use templates_delete::*;
