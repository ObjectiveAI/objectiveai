//! Disabling a provider: the daemon stops working with it — its dial
//! stopped, a dial in refused, its connection closed, and every run
//! on it taken down, each run's log keeping `undeployed` with the
//! reason `provider_disabled`. One request, one answer, and the scope
//! finishes.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
