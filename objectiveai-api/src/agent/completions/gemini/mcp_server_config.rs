//! HTTP MCP server configuration the Rust client passes to the Gemini
//! runner over stdio. The gemini runner consumes HTTP MCP servers
//! keyed by name, each with a `url` plus a flat `headers` map.
//!
//! Field names mirror the gemini runner's `mcp_servers` schema 1:1
//! (`url`, `headers`) so the Python runner can read this map straight
//! into its `_build_mcp_servers` step without a translation layer.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// HTTP MCP server config — URL plus optional HTTP headers. Mirrors
/// the gemini runner's `mcp_servers.<name>` schema field-for-field
/// (`url`, `headers`).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpServerConfig {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<IndexMap<String, String>>,
}

impl From<&objectiveai_sdk::mcp::Connection> for McpServerConfig {
    fn from(conn: &objectiveai_sdk::mcp::Connection) -> Self {
        // The connection's `headers` field is the same merged map the
        // proxy stamps on every request — User-Agent / X-Title /
        // Referer / HTTP-Referer / Authorization / any custom X-*.
        // Add `Mcp-Session-Id` on top so the SDK reuses the same
        // session.
        let mut headers = conn.headers.clone();
        if !conn.session_id.is_empty() {
            headers.insert("Mcp-Session-Id".to_string(), conn.session_id.clone());
        }

        McpServerConfig {
            url: conn.url.clone(),
            headers: if headers.is_empty() { None } else { Some(headers) },
        }
    }
}
