//! Attaching a tool to every agent of a template, by the tool's name
//! and the template's id.
//!
//! One request, one answer. A client names a tool of its own and an
//! agent template; the daemon answers that the tool is attached to the
//! template, that no tool or no template is the one named, or that it
//! failed, and the scope finishes. From the attach on, every agent
//! made from the template has the tool — those made before, running
//! now or not, and those made after — exactly as if each were
//! attached by [`attach_agent_instance`](super::attach_agent_instance):
//! a running agent's proxy serves the tool from that instant, and
//! tells the agent its tool list changed. An agent attached both ways
//! has the tool once.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer
//! is in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
