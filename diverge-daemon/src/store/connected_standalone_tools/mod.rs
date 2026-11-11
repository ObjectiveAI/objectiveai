//! The connected standalone tool records: another daemon's standalone
//! tools, held here on their own record and names.
//!
//! [`ConnectedTool`] is one as the store holds it: the daemon it is on,
//! by the ID of this daemon's record of it, the route beyond that
//! record, the tool as that daemon names it, its index among those
//! ever held from that tool, its name, its tags, who made it and when.
//! Nothing of what the tool is made from is here — that is the other
//! daemon's — and the connect this daemon holds on it is the live
//! state's. Beside the row are its [`attachments`] to agents and its
//! [`template_attachments`] to agent templates, in attach order;
//! [`attached_agents`] is every agent that has it either way. [`all`],
//! [`by_id`] and [`by_reference`] load; [`create`] holds one, its index
//! from [`counters`](crate::store::counters) and its name claimed
//! across both tool families by
//! [`tool_names`](crate::store::tool_names); [`rename`] changes its
//! name, claimed the same way; [`delete`] removes one, its attachments
//! with it; [`set_tags`] writes the tags; [`set_deployed`] keeps when
//! its connect was last taken or let go, [`set_active`] when its calls
//! in flight last began or ceased. Whether one names a remote daemon's
//! record, which refuses that record's delete, is
//! [`in_use::daemon`](crate::store::in_use::daemon).
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod activity;
mod attached_agents;
mod create;
mod delete;
mod edit;
mod load;
mod row;
mod tags;
mod tool;

pub mod attachments;
pub mod template_attachments;

pub use activity::*;
pub use attached_agents::*;
pub use create::*;
pub use delete::*;
pub use edit::*;
pub use load::*;
pub use tags::*;
pub use tool::*;
