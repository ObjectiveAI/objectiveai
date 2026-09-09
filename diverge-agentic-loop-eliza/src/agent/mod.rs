//! The agent: Eliza's parameters, as the caller states them.
//!
//! This is what the `agent` value on the container's request must
//! be, and every type here derives its JSON Schema so `GET /schema`
//! can say so — [`schemars::schema_for!`] over [`Agent`] is the
//! whole answer.
//!
//! Eliza (elizaOS, the TypeScript agent framework) is a LIBRARY the
//! harness drives from an entry of its own: one `AgentRuntime` per
//! run, its plugins the image's and the ones the caller names, its
//! memory the caller's Postgres reached through the proxy, its
//! secrets the caller's vault, its tools the caller's MCP servers
//! through the proxy. What a request can say is what that runtime
//! is constructed with — and every field here names a constructor
//! option, a character field or a plugin's setting the harness
//! actually renders. The research behind each ruling is the eliza
//! crate's `reports/2.md` (the questions), `reports/3.md` (the first
//! answers) and `reports/4.md` (the design under the containers
//! API, which this vocabulary follows where the two differ).
//!
//! What is deliberately ABSENT: `character.plugins`, a list of names
//! (nothing in the runtime honors it; a plugin is an object handed
//! to the constructor, which is what [`Plugin`] becomes); `templates`
//! (the message handler's prompt is not overridable where it
//! matters); model TIERS (Eliza takes a ladder from nano to mega, the
//! harness sets every rung to the one [`model`](Provider::model));
//! connectors, cloud, wallets, device bridges and sub-agent
//! orchestration (always off — see [`Toolsets`]); `settings` and
//! `secrets` maps on the character (every setting is rendered from
//! these structures and a plugin's own [`settings`](Plugin::settings);
//! every secret is the vault's); and any credential at all.

pub mod character;
pub mod plugin;
pub mod toolsets;

mod agent;
mod embedding;
mod memory;
mod provider;

pub use agent::*;
pub use character::Character;
pub use embedding::*;
pub use memory::*;
pub use plugin::Plugin;
pub use provider::*;
pub use toolsets::Toolsets;
