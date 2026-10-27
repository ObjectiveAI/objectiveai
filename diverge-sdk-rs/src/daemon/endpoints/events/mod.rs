//! Events: the graph of who sent to whom through the daemon.
//!
//! An EVENT is one edge of that graph: a sender and a receiver — who
//! sent, and who received. The daemon records an occurrence at three
//! seams: a message enqueued to an agent, an MCP tool call into a
//! tool, and an MCP resource read into a tool. The event does not say
//! which seam it passed: the receiver's kind implies it, a message
//! for an agent and a call or a read for a tool. Either end is a
//! [`Party`] — an account over an endpoint, an agent, a standalone
//! tool or a dependency tool — named by what never changes and never
//! by its name; a tool connected from another daemon stands as the
//! party that connected it. The graph says that the exchange happened
//! and how often, never what it carried: no message body, no
//! arguments, no result, and no time.
//!
//! # One row per edge
//!
//! One event per sender and receiver, with a `count` that grows: the
//! first occurrence adds the edge, and every one after it raises the
//! count by one. A pair that exchanges a thousand messages is one
//! event whose count is a thousand, so the graph is as large as the
//! connections it describes and no larger, however busy they are.
//!
//! # Removed with its parties
//!
//! An edge is removed when either of its parties is deleted, and at
//! no other time: an account, an agent, a standalone tool or a
//! dependency tool deleted takes every edge it is an end of, whatever
//! deleted it — its own delete, or the delete of what it went with.
//! An administrator is no account and is never deleted, so its edges
//! stay.
//!
//! [`list`] is the one request: it lists the events, narrowed — by a
//! party on either side, by sender, by receiver — the latest so many
//! by id, then keeps the scope open for every one added, changed or
//! removed after. Nothing records an event over this wire: the daemon
//! writes the graph at the seams the exchanges pass through, and a
//! client reads it. What a list reports is an [`Event`].

mod event;
mod party;

pub use event::*;
pub use party::*;

pub mod list;
