//! Getting one credential. One request, one answer. A client names a
//! credential of its own — a key credential by its identity, a hook
//! credential by its resource; the daemon answers with the credential
//! as a list would report it, without its key, that no credential is
//! the one named, or that it failed, and the scope finishes.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
