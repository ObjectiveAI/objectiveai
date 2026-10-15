//! Naming a provider to dial. One request, one answer. A client gives
//! an address and a mode; the daemon answers that the provider is
//! added, that the address is a provider's already, or that it failed,
//! and the scope finishes. From then on the daemon dials the address,
//! in that mode, for whatever it needs of the provider, and containers
//! may be pinned to it by its identity.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
