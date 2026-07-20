//! Types for the proxy-local `servers/list` aggregate — enumerating the
//! upstream MCP servers the proxy has connected for a completion, with each
//! server's initialize metadata. This is NOT a standard MCP method; the proxy
//! answers it from its in-memory connection set.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The proxy's connected upstream MCP servers and their metadata.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "mcp.ListServersResult")]
pub struct ListServersResult {
    pub servers: Vec<Server>,
}

/// One connected upstream MCP server.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "mcp.Server")]
pub struct Server {
    /// The proxy's routing prefix for this server (matches the `<prefix>_`
    /// prepended to its tools/resources in the aggregated surface).
    pub name: String,
    /// The upstream server's URL.
    pub url: String,
    /// The server's `initialize` response: capabilities, `server_info`
    /// (name, version, title, description), instructions, protocol version.
    pub initialize_result: super::initialize_result::InitializeResult,
    /// Set only when this upstream is genuinely a laboratory — i.e. a
    /// client (websocket) laboratory today. Non-laboratory servers (plain
    /// HTTP, the primary `objectiveai` MCP, plugins) leave this `None`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub laboratory: Option<crate::laboratories::Laboratory>,
    /// The laboratory's ASSISTANT-FACING composite id —
    /// `{machineID}/{base62(state)}/{base62(laboratoryID)}`
    /// ([`ClientLaboratory::composite_id`](crate::laboratories::ClientLaboratory::composite_id)),
    /// what `laboratory_transfer` takes as `source`/`destination`.
    /// Present exactly when `laboratory` is (and its marker carries
    /// the machine pair).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub laboratory_id: Option<String>,
    /// Set only when this upstream is a plugin-hosted MCP server. Other
    /// servers (plain HTTP, the primary `objectiveai` MCP, laboratories)
    /// leave this `None`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub plugin: Option<Plugin>,
}

/// A plugin MCP server's identity — the `(owner, name, version)`
/// coordinate trio (one plugin IS one MCP server; mirrors
/// `McpKind::Plugin`).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "mcp.Plugin")]
pub struct Plugin {
    pub owner: String,
    pub name: String,
    pub version: String,
}
