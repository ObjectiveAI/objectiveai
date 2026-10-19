//! The event records: the graph of who sent what to whom through the
//! daemon.
//!
//! [`Event`] is one edge as the store holds it — a sender, a receiver,
//! the action between them and how many times it has passed; [`all`]
//! loads every one, oldest first; [`record`] is the one write, an
//! occurrence that adds the edge or raises its count, in whatever
//! connection the caller holds. Nothing removes one, since nothing
//! undoes an occurrence. The action is kept as its snake-case word,
//! which [`action_name`] and [`action_of`] translate, the same word
//! the column's constraint admits.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod event;
mod load;
mod record;

pub use event::*;
pub use load::*;
pub use record::*;
