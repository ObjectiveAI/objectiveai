//! Discriminator identifying which CLI-hosted MCP server a frame
//! addresses. Lives at the module root so [`super::server_request`],
//! [`super::server_response`], and [`super::client_request`] can
//! all reach it without circular `super::super::` chains.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Which CLI-hosted MCP server a frame belongs to. Stamped on every
/// [`super::server_request::Request`] / [`super::server_response::Response`]
/// envelope and on every [`super::client_request::McpListChanged`]
/// payload so the CLI's per-MCP dispatch table can route by enum
/// rather than by parsing the proxy's URL path on every hop.
///
/// Wire shape: `{"type":"objective_ai"}`,
/// `{"type":"plugin","owner":"…","name":"…","version":"…"}`, or
/// `{"type":"laboratory","id":"…"}`.
///
/// The coordinate trio on `Plugin` mirrors the API's
/// `/{owner}/{name}/{version}` URL path that the proxy dials for
/// plugin MCP servers — one plugin IS one MCP server.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema,
)]
#[schemars(rename = "client_objectiveai_mcp.McpKind")]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum McpKind {
    /// The CLI's primary `objectiveai-mcp` upstream — the local MCP
    /// server bound at the CLI process's `--mcp-address`.
    #[schemars(title = "ObjectiveAI")]
    ObjectiveAi,

    /// A plugin's MCP server — one plugin IS one MCP server,
    /// identified by the plugin's `(owner, name, version)` coordinate
    /// trio. Mirrors the API URL `/{owner}/{name}/{version}` and the
    /// agent declaration's `plugins[i].{owner,name,version}`.
    #[schemars(title = "Plugin")]
    Plugin {
        owner: String,
        name: String,
        version: String,
    },

    /// A laboratory-hosted MCP server, identified by an opaque `id`.
    /// Mirrors the proxy URL `client://laboratory/{id}`. Laboratory ids
    /// are only unique per (machine, state); `machine` +
    /// `machine_state` pin the exact laboratory host so the CLI
    /// conduit forwards precisely — an absent pair falls back to
    /// legacy first-match-by-id resolution.
    #[schemars(title = "Laboratory")]
    Laboratory {
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        machine: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        machine_state: Option<String>,
        /// For agent-embedded laboratories: the seed the CLI conduit
        /// needs to CREATE the laboratory when no connected host
        /// serves the derived `id` yet (reuse needs only the id).
        /// `None` for user-created laboratories.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        agent: Option<AgentLaboratorySeed>,
    },
}

/// The seed of an agent-embedded laboratory, riding
/// [`McpKind::Laboratory`]: the source agent's full id plus the
/// embedded spec — everything the CLI conduit's on-the-fly create
/// needs.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema,
)]
#[schemars(rename = "client_objectiveai_mcp.AgentLaboratorySeed")]
pub struct AgentLaboratorySeed {
    /// The full id of the agent the laboratory derives from.
    pub agent_full_id: String,
    /// The embedded laboratory spec (image, env, cwd).
    pub laboratory: crate::agent::Laboratory,
}
