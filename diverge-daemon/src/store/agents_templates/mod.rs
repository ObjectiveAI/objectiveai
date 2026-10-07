//! The agent template records.
//!
//! [`Record`] is one as the store holds it: the template whole, under
//! the id that is its hash, with its tags and who first made it. A
//! template deleted stays as a row with `deleted` set and no tags, so
//! that one made anew after the delete is the one that was deleted,
//! its creator and its created time the first create's — as the wire
//! says it is. [`all`] and [`by_id`] load the live ones; [`create`]
//! makes one or revives one, answering whether it was live already;
//! [`delete`] marks one; [`set_tags`] writes the tags.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod create;
mod delete;
mod load;
mod record;
mod row;
mod tags;

pub use create::*;
pub use delete::*;
pub use load::*;
pub use record::*;
pub use tags::*;
