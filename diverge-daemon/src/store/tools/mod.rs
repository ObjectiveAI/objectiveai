//! The tool records: what a tool is, apart from its container.
//!
//! [`Tool`] is one as the store holds it, of one of two
//! [`Origin`]s: made from a template, with what its create added, or
//! joined to somebody else's container by its provider, its id and an
//! authorization. Beside the row are its [`attachments`] to agents, in
//! attach order; its [`admissions`], who may see it from its provider
//! and who may join it; and the routes that name it, which are
//! [`routes`](crate::store::routes)'. The container is work made from
//! the row and is nowhere in it. [`all`], [`by_id`] and
//! [`by_reference`] load; [`create`] makes one, its index from
//! [`counters`](crate::store::counters); [`update`] rewrites what an
//! edit may change; [`delete`] removes one, its attachments, admissions
//! and routes with it; [`set_tags`] writes the tags; [`set_last`] keeps
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

pub mod admissions;
pub mod attachments;

pub use activity::*;
pub use create::*;
pub use delete::*;
pub use edit::*;
pub use load::*;
pub use tags::*;
pub use tool::*;
