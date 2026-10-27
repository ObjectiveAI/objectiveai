//! The event records: the graph of who sent to whom through the
//! daemon.
//!
//! [`Event`] is one edge as the store holds it — a sender, a receiver,
//! and how many times something has passed between them, each end a
//! [`Party`](diverge_sdk::daemon::endpoints::events::Party); [`all`]
//! loads every one, oldest first, each end joined to the row it names;
//! [`record`] is the one write, an occurrence that adds the edge or
//! raises its count, in whatever connection the caller holds, each end
//! resolved to the row it names. Nothing here removes one: an
//! end that is an agent, a standalone tool or a dependency tool
//! references its row, whose delete cascades to the edge, and an
//! account's delete takes the edges of its identity itself.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod event;
mod load;
mod party;
mod record;

pub use event::*;
pub use load::*;
pub use record::*;
