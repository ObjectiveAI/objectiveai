//! Detaching a tool from an agent, by their names.
//!
//! One request, one answer. A client names a tool and an agent of its
//! own; the daemon answers that the tool is detached from the agent,
//! that no tool or no agent is the one named, that the agent is active
//! and was left as it is, or that it failed, and the scope finishes.
//! A detach lands only on an idle agent: an agent with a loop running
//! keeps its tools until the loop ends.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer
//! is in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
