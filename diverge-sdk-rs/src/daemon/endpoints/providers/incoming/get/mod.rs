//! Getting one credential. One request, one answer. A client names a
//! credential by its identity; the daemon answers with it as a list
//! would report it, which carries no key, that no credential names that
//! identity, forbidden, or that it failed, and the scope finishes.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
