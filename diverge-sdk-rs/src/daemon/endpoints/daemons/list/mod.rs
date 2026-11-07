//! Listing the remote daemons this daemon has records of. A client asks
//! for them, narrowed — by ID, by the remote's issuer, by creator, by
//! tags, by when they were added, by whether this daemon holds a
//! connection to one now — and the daemon sends one response each, by
//! ID, then the word that the list is whole, and keeps the scope open:
//! each record added, changed — its fields, its tags, its connection
//! coming and going — or removed, until the client cancels, the one
//! channel it opens on the scope. A count keeps the list to the first
//! that many that match.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
