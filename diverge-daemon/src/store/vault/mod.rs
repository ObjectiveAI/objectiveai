//! The vault records: one JSON value per key.
//!
//! [`Entry`] is one as the store holds it, with the agents and tools
//! binding it beside; [`all`] and [`by_key`] load; [`set`],
//! [`set_tags`] and [`delete`] write, each inside whatever
//! transaction the caller holds; [`values`] reads the values a run
//! start hands its container. A key some record binds is not
//! deleted: the binding lives inside the record's `secrets`, which
//! no constraint reaches, so [`delete`] asks
//! [`in_use`](crate::store::in_use) first and answers as a value.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod delete;
mod entry;
mod load;
mod row;
mod set;
mod tags;
mod values;

pub use delete::*;
pub use entry::*;
pub use load::*;
pub use set::*;
pub use tags::*;
pub use values::*;
