//! Wire shape for a plugin-emitted MCP server announcement.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Emitted by a running plugin to announce an MCP server URL it wants
/// the host to expose. The host routes this through the standard
/// plugin-notification pipeline and dials the URL the same way it would
/// for an entry in the plugin's manifest `mcp_servers` — runtime
/// announcements are functionally identical to manifest-time
/// declarations.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
#[schemars(rename = "cli.plugins.Mcp")]
pub struct Mcp {
    pub url: String,
}
