//! The dependency template records: what an agent's program declares,
//! held by its hash, and what each agent depends on now.
//!
//! [`Record`] is one as the store holds it: the template whole, under
//! the id that is its hash, with its tags and the agent whose
//! registration first declared it. A template deleted stays as a row
//! with `deleted` set and no tags, so that one declared anew after the
//! delete is the one that was deleted, its creator and its created
//! time the first registration's — as the wire says it is. [`all`]
//! and [`by_id`] load the live ones; [`create`] makes one or revives
//! one, answering whether it was live already; [`delete`] marks one;
//! [`set_tags`] writes the tags. Beside the records, [`agents`] is
//! what each agent depends on: exactly the templates its last
//! registration declared, reset at every registration.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod create;
mod delete;
mod load;
mod record;
mod row;
mod tags;

pub mod agents;

pub use create::*;
pub use delete::*;
pub use load::*;
pub use record::*;
pub use tags::*;
