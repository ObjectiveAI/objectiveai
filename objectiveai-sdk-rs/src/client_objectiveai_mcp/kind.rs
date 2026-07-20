//! Discriminator identifying which CLI-hosted MCP server a frame
//! addresses. Lives at the module root so [`super::server_request`],
//! [`super::server_response`], and [`super::client_request`] can
//! all reach it without circular `super::super::` chains.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Which client-side MCP server a frame belongs to. Stamped on every
/// [`super::server_request::Request`] / [`super::server_response::Response`]
/// envelope and on every [`super::client_request::McpListChanged`]
/// payload so the CLI's per-MCP dispatch table can route by enum
/// rather than by parsing the proxy's URL path on every hop.
///
/// Wire shape: `{"type":"plugin_laboratory","owner":"…",…}`,
/// `{"type":"laboratory","id":"…",…}`, or
/// `{"type":"agent_laboratory","id":"…",…}` — all three kinds are
/// EPHEMERAL containers on laboratory hosts (agents are plugin-only;
/// there is no daemon-hosted MCP server anymore).
///
/// The three kinds are deliberately DISTINCT variants rather than one
/// laboratory variant with optional fields: a user-created laboratory
/// is pinned by (machine, state), an agent-embedded laboratory is
/// created on the fly from its seed, and a plugin laboratory is named
/// by its coordinate trio — they share nothing but the container
/// substrate.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema,
)]
#[schemars(rename = "client_objectiveai_mcp.McpKind")]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum McpKind {
    /// A plugin's MCP server — one plugin IS one MCP server,
    /// identified by the plugin's `(owner, name, version)` coordinate
    /// trio. Mirrors the API URL `/{owner}/{name}/{version}` and the
    /// agent declaration's `plugins[i].{owner,name,version}`.
    #[schemars(title = "PluginLaboratory")]
    PluginLaboratory {
        owner: String,
        name: String,
        version: String,
    },

    /// A USER-CREATED laboratory's MCP server, identified by an opaque
    /// `id`. Laboratory ids are only unique per (machine, state);
    /// `machine` + `machine_state` pin the exact laboratory host so
    /// the CLI conduit forwards precisely — an absent pair falls back
    /// to legacy first-match-by-id resolution.
    #[schemars(title = "Laboratory")]
    Laboratory {
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        machine: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        machine_state: Option<String>,
    },

    /// An AGENT-EMBEDDED laboratory's MCP server. `id` is the derived
    /// `oai-agent-…` laboratory id; the seed fields are everything the
    /// CLI conduit needs to CREATE the laboratory when no connected
    /// host serves the derived `id` yet (reuse needs only the id).
    /// Never pinned to a (machine, state) — the conduit resolves or
    /// creates on whichever host it picks.
    #[schemars(title = "AgentLaboratory")]
    AgentLaboratory {
        id: String,
        /// The full id of the agent the laboratory derives from.
        agent_full_id: String,
        /// The embedded laboratory spec (image, env, cwd).
        laboratory: crate::agent::Laboratory,
    },
}

