//! Listing the daemon's providers. A client asks for them, narrowed —
//! by identity, by whether one is connected now — and the daemon sends
//! one response each, in the configuration's order, then the word that
//! the list is whole, and keeps the scope open: each provider changed
//! as its connection and its capacity change, until the client
//! cancels, the one channel it opens on the scope. Each is its
//! identity, the address it is dialled at where it is dialled, where
//! the daemon stands with it, and its capacity. A count keeps the list
//! to the first that many that match.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
