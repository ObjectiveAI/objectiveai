//! Events: the graph of who sent what to whom through the daemon.
//!
//! An EVENT is one edge of that graph: a sender and a receiver — who
//! sent, and who received. The daemon records an occurrence at three
//! seams: a message enqueued to an agent, an MCP tool call into a
//! tool, and an MCP resource read into a tool. The event does not say
//! which seam it passed: the receiver's kind implies it, a message
//! for an agent and a call or a read for a tool. Either end is
//! a [`Creator`](crate::daemon::creator::Creator) — the client over
//! an endpoint, or an agent or a tool of the client's — which is who
//! the daemon knows to have acted; a tool connected from another
//! daemon stands as the creator that connected it. The graph says
//! that the exchange happened and how often, never what it carried:
//! no message body, no arguments, no result, and no time.
//!
//! # One row per edge
//!
//! One event per sender and receiver, with a `count` that
//! grows: the first occurrence adds the edge, and every one after it
//! raises the count by one. A pair that exchanges a thousand messages
//! is one event whose count is a thousand, so the graph is as large
//! as the connections it describes and no larger, however busy they
//! are. It follows that an edge is never removed — nothing undoes an
//! occurrence — so a list of events is added to and changed, never
//! taken from.
//!
//! [`list`] is the one request: it lists the events, narrowed — by a
//! party on either side, by sender, by receiver — the
//! latest so many by id, then keeps the scope open for every one
//! added or changed after. Nothing records an event over this wire:
//! the daemon writes the graph at the seams the exchanges pass
//! through, and a client reads it. What a list reports is an
//! [`Event`].

mod event;

pub use event::*;

pub mod list;
