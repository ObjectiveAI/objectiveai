//! Agent definitions — what a provider USED to be asked to run.
//!
//! Held for reference, and not on the wire. The
//! [`agent`](crate::endpoints::containers::agents::run::client::request::Frame::agent)
//! on an agent container's request is a JSON value the image defines;
//! these are the typed shapes that value had when this crate named the
//! agents itself, kept so the next thing that needs one can start from
//! them. An image that has been brought up to the containers API owns
//! its agent itself and leaves here: the openrouter agent lives in
//! `diverge-agentic-loop-openrouter` now and the Claude Code agent in
//! `diverge-agentic-loop-cc`, each beside the loop that reads it and
//! the schema it states.
//!
//! POST-TRANSFORM. An agent as authored carries things that shape a
//! request before a provider ever sees it: a system prompt, prefix and
//! suffix messages, a personality. Those are applied by whoever builds
//! the request, and what arrives here is the result — so nothing in
//! this module rewrites a conversation. What remains is the parameters
//! of the call itself, which only the upstream can interpret.
//!
//! Provisioning is likewise absent. MCP servers, laboratories and
//! plugins decide what an agent CAN reach, which is settled before a
//! request is built rather than declared inside one.

pub mod codex;
pub mod eliza;
pub mod hermes;
pub mod python;

mod agent;

pub use agent::*;
