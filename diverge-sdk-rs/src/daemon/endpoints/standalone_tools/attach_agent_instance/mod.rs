//! Attaching a tool to one agent, by their names.
//!
//! One request, one answer. A client names a tool and an agent of its
//! own; the daemon answers that the tool is attached to the agent,
//! that no tool or no agent is the one named, or that it failed, and the
//! scope finishes. An attach lands on an active agent as well as an
//! idle one: the agent's proxy serves the tool from that instant, and
//! tells the agent its tool list changed. An agent has a tool also
//! through its template — see
//! [`attach_agent_template`](super::attach_agent_template) — and has
//! it once however many ways it is attached.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer
//! is in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
