//! Getting one of the host's identities. One request, one answer. A
//! client names an identity by its issuer and its subject; the daemon
//! answers with it as a list would report it — its tags, and no token —
//! that no identity on the host is the one named, forbidden, or that it
//! failed, and the scope finishes. The files are read as they are then.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
