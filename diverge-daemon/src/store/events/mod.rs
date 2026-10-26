//! The event records: the graph of who sent to whom through the
//! daemon.
//!
//! [`Event`] is one edge as the store holds it — a sender, a receiver,
//! and how many times something has passed between them; [`all`]
//! loads every one, oldest first; [`record`] is the one write, an
//! occurrence that adds the edge or raises its count, in whatever
//! connection the caller holds. Nothing removes one, since nothing
//! undoes an occurrence.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod event;
mod load;
mod record;

pub use event::*;
pub use load::*;
pub use record::*;
