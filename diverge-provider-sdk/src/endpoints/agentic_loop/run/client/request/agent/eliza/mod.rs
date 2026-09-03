//! Eliza agent parameters.
//!
//! Eliza (elizaOS, the TypeScript agent framework) is a LIBRARY the
//! harness drives from an entry of its own: one `AgentRuntime` per
//! container, its plugins the image's, its memory the PGlite
//! database that travels as the continuation, its tools the
//! caller's MCP servers through the proxy. What a request can say
//! is what that runtime is constructed with — and every field here
//! names a constructor option, a character field or a plugin's
//! setting the harness actually renders. The research behind each
//! ruling is the eliza crate's `reports/2.md` (the questions) and
//! `reports/3.md` (the answers).
//!
//! What is deliberately ABSENT: a `plugins` list of names (nothing
//! in the host honors `character.plugins`, and a plugin the image
//! lacks resolves to nothing — Eliza's plugins are the harness's,
//! switched in [`Toolsets`] and [`Memory`]; the caller's
//! extensibility is MCP); `templates` (the message handler's prompt
//! is not overridable where it matters); model TIERS (Eliza takes a
//! ladder from nano to mega, the harness sets every rung to the one
//! [`model`](Provider::model)); connectors, cloud, wallets, device
//! bridges and sub-agent orchestration (always off — see
//! [`Toolsets`]); `settings`/`secrets` maps (every setting is
//! rendered from these structures, and a free map would be the
//! credential channel the protocol closed).

pub mod character;
pub mod toolsets;

mod agent;
mod embedding;
mod memory;
mod provider;
mod upstream;

pub use agent::*;
pub use character::Character;
pub use embedding::*;
pub use memory::*;
pub use provider::*;
pub use toolsets::Toolsets;
pub use upstream::*;
