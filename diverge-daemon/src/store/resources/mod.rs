//! The resource records: what is known about bytes the content store
//! holds.
//!
//! [`Record`] is one as the store holds it: the id that is the hash
//! of the bytes, which kind, the latest description, the size, the
//! tags, and who first held it. A resource deleted stays as a row
//! with `deleted` set and no tags, so that the same bytes held anew
//! are the one that was deleted, its creator and created time the
//! first upload's — as the wire says. [`all`] and [`by_id`] load the
//! live ones; [`hold`] makes a record, revives one, or finds one live
//! and gives it the latest description; [`delete`] marks one;
//! [`set_tags`] writes the tags. The bytes are
//! [`content`](crate::content)'s.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod delete;
mod hold;
mod load;
mod record;
mod row;
mod tags;

pub use delete::*;
pub use hold::*;
pub use load::*;
pub use record::*;
pub use tags::*;
