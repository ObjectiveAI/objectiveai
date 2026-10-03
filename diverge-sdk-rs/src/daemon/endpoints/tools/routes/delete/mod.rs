//! Taking a route up. One request, one answer. A client names a
//! position; the daemon answers that the route is gone, that the
//! position has none, that an active container is served through it and
//! it was left as it is, or that it failed, and the scope finishes. A
//! position without a route is answered by the deployer again.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
