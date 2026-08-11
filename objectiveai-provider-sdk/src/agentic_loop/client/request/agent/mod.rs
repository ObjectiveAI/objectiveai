//! Agent definitions — what a provider is asked to run.
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

pub mod claude_agent_sdk;
pub mod codex_sdk;
pub mod openrouter;
pub mod python;

mod agent;

pub use agent::*;
