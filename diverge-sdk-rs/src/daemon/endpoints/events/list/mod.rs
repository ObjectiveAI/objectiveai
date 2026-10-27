//! Listing events. A client asks for the edges of the graph, narrowed,
//! and the daemon, when the account holds `list` over events, sends
//! every one the request's filter lets through — by a party on either
//! side, by sender, by receiver — the latest `count` of them by id,
//! one response each, then the word that the list is whole, and keeps
//! the scope open: every event added, every one whose count is
//! raised, and every one sent whose party was deleted, after, with no
//! cap, until the client cancels, the one channel it opens on the
//! scope. Each is the [`Event`](super::Event): the sender, the
//! receiver and the count. A request that says nothing is every
//! event. A caller
//! whose filter matches nothing is told the list is whole at once,
//! and watched.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
