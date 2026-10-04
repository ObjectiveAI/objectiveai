//! Forgetting an outgoing provider. One request, one answer. A client
//! names a provider of its own by address; the daemon answers that the
//! provider is forgotten, that no provider of the caller's has that
//! address, that a container of the caller's is pinned to it or mounts
//! or serves a volume of its and it was left as it is, or that it
//! failed, and the scope finishes. A forgotten provider is dialled no
//! more, and its address is free for an add.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
