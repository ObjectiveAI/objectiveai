//! Editing a provider's record, by its identity: every member given
//! replaces the record's. A change to its direction or its listable
//! flag closes its connection, and the daemon reaches it again as the
//! record now stands. One request, one answer, and the scope finishes.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
