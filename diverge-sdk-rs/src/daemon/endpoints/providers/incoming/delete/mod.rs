//! Taking a credential out. One request, one answer. A client names a
//! credential by its identity; the daemon answers that the credential
//! is gone, that no credential names that identity, that a provider is
//! connected through it and it was left as it is, forbidden, or that it
//! failed, and the scope finishes. The key the credential answered
//! admits nothing from then on.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
