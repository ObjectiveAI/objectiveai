//! Deleting a remote daemon's record. Refused while a connected tool
//! names it: those are deleted first. A connection held to it is
//! dropped.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
