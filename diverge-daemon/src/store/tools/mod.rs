//! The tool records: what a tool is, apart from its container.
//!
//! [`Tool`] is one as the store holds it, of one of two
//! [`Origin`]s: made from a template, with what its create added, or
//! joined to another daemon's tool by the daemon's record and the tool
//! as that daemon names it. Beside the row are its [`attachments`] to
//! agents, in attach order. The container is work made from the row and
//! is nowhere in it; a dependency tool, deployed for an agent, has no
//! row at all and is the live state's. [`all`], [`by_id`] and
//! [`by_reference`] load; [`create`] makes one, its index from
//! [`counters`](crate::store::counters); [`update`] rewrites what an
//! edit may change; [`delete`] removes one, its attachments with it;
//! [`set_tags`] writes the tags; [`set_last`] keeps
//! where and when it last ran.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod activity;
mod create;
mod delete;
mod edit;
mod load;
mod row;
mod tags;
mod tool;

pub mod attachments;

pub use activity::*;
pub use create::*;
pub use delete::*;
pub use edit::*;
pub use load::*;
pub use tags::*;
pub use tool::*;
