//! The agent records: what an agent is, apart from its container.
//!
//! [`Agent`] is one as the store holds it: a template and what the
//! create added — a name, a provider and its volumes, mounts across
//! providers; no account, since an agent asks the daemon nothing —
//! with its once-and-for-all
//! index, the provider it last ran on and when, its tags, who made
//! it. The container is work made from the row and is nowhere in it;
//! what runs is the live state's. [`all`], [`by_id`] and
//! [`by_reference`] load; [`create`] makes one, its index from
//! [`counters`](crate::store::counters); [`update`] rewrites what an
//! edit may change; [`delete`] removes one, its attachments with it;
//! [`set_tags`] writes the tags; [`set_deployed`] keeps where and when
//! its container last came up or went down, [`set_active`] when its
//! loop last began or ended.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod activity;
mod agent;
mod create;
mod delete;
mod edit;
mod load;
mod row;
mod tags;

pub use activity::*;
pub use agent::*;
pub use create::*;
pub use delete::*;
pub use edit::*;
pub use load::*;
pub use tags::*;
