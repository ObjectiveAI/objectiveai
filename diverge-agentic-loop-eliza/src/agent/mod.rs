//! The agent: Eliza's parameters, as the caller states them.
//!
//! This is what the `arguments` on the container's request must
//! be, and every type here derives its JSON Schema so `GET /schema`
//! can say so — [`schemars::schema_for!`] over [`Agent`] is the
//! whole answer. One member is not this image's to define:
//! `mcp_tools`, the tool containers the agent depends on, in the form
//! the provider's wire gives a tool, passed back whole as the
//! registration's answer.
//!
//! Eliza (elizaOS, the TypeScript agent framework) is a LIBRARY the
//! harness drives from an entry of its own: one `AgentRuntime` per
//! run, its memory the caller's Postgres reached through the proxy,
//! its secrets the caller's vault, its tools the caller's MCP servers
//! through the proxy, and its plugins the ones the caller names. What
//! a request can say is what that runtime is constructed with — and
//! every field here names a constructor option, a character field or
//! a plugin the harness actually loads. The research behind each
//! ruling is the eliza crate's `reports/2.md` (the questions),
//! `reports/3.md` (the first answers) and `reports/4.md` (the design
//! under the containers API, which this vocabulary follows where the
//! two differ).
//!
//! # Two lists of plugins, and the rule between them
//!
//! [`model_provider_plugins`](Agent::model_provider_plugins) holds
//! every plugin that registers a MODEL HANDLER — what Eliza calls a
//! model-provider plugin: the model that speaks, the one that embeds,
//! the one that describes an image or transcribes audio — in priority
//! order, the first listed answering every model type it registers
//! and each later one the failover behind it. [`plugins`](Agent::plugins)
//! holds everything else. The rule is ENFORCED, twice: when a package
//! is imported, one under `plugins` that declares model handlers, or
//! one under `model_provider_plugins` that declares none, refuses the
//! run naming the package and the list it belongs in; and after the
//! runtime initializes, every model handler it holds must belong to a
//! model-provider plugin or the adapter, or the run is refused naming
//! the handler — so a plugin that registers a model from its `init`
//! is caught too. There is no way to make a model provider of a plugin
//! listed in the wrong place.
//!
//! # Pre-installed is the only privilege
//!
//! The image carries `@elizaos/plugin-openai`, `plugin-embeddings`,
//! `plugin-coding-tools`, `plugin-browser`, `plugin-documents`,
//! `plugin-web-search` and `vault` at its pin, already installed. That
//! is their whole difference from a package the registry serves: a
//! bare spec for one loads the image's copy without an install, a
//! spec naming another version installs that version, and one that is
//! not listed is not loaded and nothing is rendered for it. Their
//! settings are their own vocabulary — `OPENAI_BASE_URL` and the
//! `OPENAI_*_MODEL` tiers, `EMBEDDING_BASE_URL`, `EMBEDDING_MODEL` and
//! `EMBEDDING_DIMENSIONS`, `CODING_TOOLS_WORKSPACE_ROOTS`,
//! `DOCUMENTS_PATH` — given as each entry's [`settings`](Plugin::settings),
//! and their secrets — `OPENAI_API_KEY`, `EMBEDDING_API_KEY`,
//! `TAVILY_API_KEY` — are named by the caller as each entry's
//! [`secrets`](Plugin::secrets), the vault's keys. The harness implies
//! no key and renders no setting of any plugin's.
//!
//! What the harness loads on its own: `@elizaos/plugin-sql`, the
//! database adapter, with `POSTGRES_URL` the proxy's pgwire; and its
//! diverge plugin, which registers each of the caller's MCP tools as
//! a native action and the resources as a provider.
//!
//! What is deliberately ABSENT: `character.plugins`, a list of names
//! (nothing in the runtime honors it; a plugin is an object handed to
//! the constructor, which is what a [`Plugin`] becomes); `templates`
//! (the message handler's prompt is not overridable where it
//! matters); `settings` and `secrets` maps on the character (every
//! setting is a plugin's own, every secret the vault's); and any
//! credential at all.
//!
//! Always off, no field, and the reason for each so nobody re-opens
//! it without a new fact — these are Eliza's own host machinery, which
//! no plugin entry can wire because they need the host's plumbing
//! around them, not a setting:
//! - CONNECTORS (discord, telegram, x, slack, whatsapp, wechat,
//!   matrix, imessage, instagram, google-workspace): a second inbound
//!   channel the queue cannot see, and turns nobody drives. An agent
//!   that lives on X gets an X MCP server from its caller and posts by
//!   tool call, on the stream.
//! - CLOUD AND HOST (elizacloud, cloud-apps, app-control, app-manager,
//!   registry, native-filesystem, the `native-*` device bridges,
//!   capacitor, omarchy, companion): another product's plumbing.
//! - LIFE-OPS (personal-assistant, goals, todos, inbox, reminders,
//!   health, finances, blocker, feed): bound to connectors,
//!   proactivity and the host UI.
//! - SUB-AGENTS AND ORCHESTRATION (agent-orchestrator, workflow):
//!   background children outliving the run, off every stream it has.
//! - LOCAL INFERENCE (local-inference, native-inference, native-llama,
//!   cli-inference): the container runs no model of its own.
//! - `pty`, `scheduling`, `computeruse`, `vision`, `notes`,
//!   trajectory-logger, form, video, fish-audio, local-storage,
//!   inmemorydb: diagnostics, device features, host UI views, or
//!   alternatives to what the harness already decides.

pub mod character;
pub mod plugin;

mod agent;
mod memory;

pub use agent::*;
pub use character::Character;
pub use memory::*;
pub use plugin::Plugin;
