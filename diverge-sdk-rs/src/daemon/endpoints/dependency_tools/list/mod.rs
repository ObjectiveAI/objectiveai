//! Listing a caller's dependency tools.
//!
//! A client asks for its dependency tools, narrowed, and the daemon
//! sends every one it holds under the caller's identity that the
//! request's filter lets through — by template, by owner, by creator,
//! by type, by scope, by whether any agent depends on it, by the agents
//! that do, by whether it runs, by tags, all or any, by when it was
//! made — one response each, oldest made first, then the word that the
//! list is whole, and keeps the scope open: each tool added, changed or
//! removed, as registrations make tools and reset what agents depend
//! on, as tools are tagged and deleted, and as their containers start
//! and stop, until the client cancels, the one channel it opens on the
//! scope. Each is the tool by its template and its owner, when it was
//! made, who made it, its tags, the template whole, the agents whose
//! last registration declared it and when each last did, and its run,
//! if its container is up. A count keeps the list to the first that
//! many that match. A request that says nothing is every tool. A
//! caller with no tool that matches is told the list is whole at once,
//! and watched.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
