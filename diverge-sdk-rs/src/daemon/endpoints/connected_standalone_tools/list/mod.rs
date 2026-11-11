//! Listing a caller's connected tools.
//!
//! A client asks for its connected tools, narrowed, and the daemon
//! sends every one it holds under the caller's identity that the
//! request's filter lets through — by name, by the daemon it is on,
//! by creator, by activity, by whether its connect is held, by the
//! agents it is attached to, by tags, all or any, by when it was
//! created — one response each, oldest created first, then the word
//! that the list is whole, and keeps the scope open: each tool added,
//! changed or removed, as the records, the attachments and the
//! connects change — until the client cancels, the one channel it
//! opens on the scope. Each is what the tool is called, the daemon it
//! is on, the route to it and the tool there, its index, whether it is
//! active now, when that last changed, which agents it is attached to,
//! and its tags. A count keeps the list to the first that many that
//! match. A request that says nothing is every connected tool. A
//! caller with no tool that matches is told the list is whole at once,
//! and watched. A standalone tool the daemon runs itself is listed by
//! the
//! [`standalone_tools`](crate::daemon::endpoints::standalone_tools)
//! list, and never here.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
