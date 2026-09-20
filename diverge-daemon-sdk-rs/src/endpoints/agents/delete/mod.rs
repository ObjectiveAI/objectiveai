//! Deleting an agent by name.
//!
//! One request, one answer. A client names an agent of its own; the
//! daemon answers that the agent is deleted, that no agent has that
//! name, that the agent is active and was left as it is, or that it
//! failed, and the scope finishes. A deleted agent's container is
//! stopped and its name is free for a [`create`](super::create).
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer
//! is in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
