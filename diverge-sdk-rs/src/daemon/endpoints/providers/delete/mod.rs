//! Deleting a provider's record, by its identity: only a disabled one,
//! and only when no agent or tool is pinned to it or mounts its
//! volumes; its volumes' tags go with it. One request, one answer, and
//! the scope finishes.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
