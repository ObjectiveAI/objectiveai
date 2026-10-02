//! Getting one agent.
//!
//! One request, one answer. A client names a agent of its own; the
//! daemon answers with the agent as a list would report it, that no
//! agent of the caller's is the one named, or that it failed, and the
//! scope finishes. The agent is named by its name, or by its template
//! and its index, as [`reference`](crate::daemon::reference) states.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer
//! is in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
