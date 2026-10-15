//! Getting one outgoing provider. One request, one answer. A client
//! names a provider of its own by address; the daemon answers with the
//! provider as a list would report it, that no provider of the caller's
//! has that address, or that it failed, and the scope finishes.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
