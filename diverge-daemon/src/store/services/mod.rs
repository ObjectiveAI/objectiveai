//! The service records: a dependency tool template an agent's program
//! declared as a service, made under its scope key and kept.
//!
//! [`Service`] is one as the store holds it: the template it was made
//! from, by id and whole, its scope and the owner the scope key names
//! — the one agent, the one agent template, or nobody for a global
//! service — who first declared it and when. Beside the row are its
//! [`attachments`] to agents, in attach order: every agent whose last
//! registration declared the template, which is what holds the
//! service up; and [`owners`], the agent templates whose agents ever
//! attached a global service, which is what holds its database scope
//! up. The container is work made from the row and is nowhere in it;
//! what runs is the live state's. [`all`], [`by_id`], [`by_scope`],
//! [`of_agent`] and [`of_owner_template`] load; [`find_or_create`]
//! makes one under its scope key, or finds the one there is;
//! [`delete`] and [`delete_of_owner_template`] remove; [`set_last`]
//! keeps where and when it last ran.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod activity;
mod create;
mod delete;
mod load;
mod row;
mod service;

pub mod attachments;
pub mod owners;

pub use activity::*;
pub use create::*;
pub use delete::*;
pub use load::*;
pub use service::*;
