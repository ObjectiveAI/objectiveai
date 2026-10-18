//! Detaching a tool from every agent of a template, by the tool's
//! name and the template's id.
//!
//! One request, one answer. A client names a tool of its own and an
//! agent template; the daemon answers that the tool is detached from
//! the template, that no tool or no template is the one named, or
//! that it failed, and the scope finishes. From the detach on, no
//! agent has the tool through the template: a running agent's proxy
//! serves it no more from that instant, active or idle, and tells the
//! agent its tool list changed; a call in flight to the tool is
//! answered still. An agent attached to the tool directly, by
//! [`attach_agent_instance`](super::attach_agent_instance), keeps it.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer
//! is in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
