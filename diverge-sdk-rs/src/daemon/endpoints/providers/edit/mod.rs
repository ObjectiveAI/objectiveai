//! Editing a provider's record, by its identity: every member given
//! replaces the record's — its announcement, whether the daemon accepts
//! other daemons and hears messages through it, an incoming record's
//! IPs, and its description. Its identity, its direction and an
//! outgoing record's address are not editable. Nothing an edit changes
//! closes the provider's connection, except `ips` that no longer admit
//! the live connection's source address: the daemon changes what it
//! holds open on the connection as it stands — see the
//! [request](client::request::Frame). One request, one answer, and the
//! scope finishes.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
