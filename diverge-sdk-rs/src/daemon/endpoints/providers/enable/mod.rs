//! Enabling a provider: the daemon works with it again — dials it when
//! it is outgoing, admits it when it is incoming. One request, one
//! answer, and the scope finishes.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
