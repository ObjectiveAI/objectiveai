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
/// Wire shape: `{"type":"objective_ai"}` or
/// `{"type":"other","owner":"…","name":"…","version":"…","mcp":"…"}`.
///
/// The four discriminator fields on `Other` mirror the API's
/// `/{owner}/{name}/{version}/{mcp}` URL path that the proxy dials
/// for plugin-hosted MCP servers.
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

    /// A plugin-hosted MCP server. Identified by the plugin's
    /// `(owner, name, version)` tuple plus the `mcp` server name
    /// within that plugin's manifest. Mirrors the API URL
    /// `/{owner}/{name}/{version}/{mcp}` and the agent declaration's
    /// `plugins[i].{owner,name,version}.mcp_servers[j].name`.
    #[schemars(title = "Other")]
    Other {
        owner: String,
        name: String,
        version: String,
        mcp: String,
    },
}
