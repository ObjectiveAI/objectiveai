//! Detaching a tool from one agent, by their names.
//!
//! One request, one answer. A client names a tool and an agent of its
//! own; the daemon answers that the tool is detached from the agent,
//! that no tool or no agent is the one named, or that it failed, and
//! the scope finishes. A detach lands on an active agent as an attach
//! does: from that instant the agent's proxy serves the tool no more
//! and tells the agent its tool list changed; a call of the agent's
//! in flight to the tool is answered still. An agent attached to the
//! tool through its template as well keeps it: see
//! [`detach_agent_template`](super::detach_agent_template).
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer
//! is in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
