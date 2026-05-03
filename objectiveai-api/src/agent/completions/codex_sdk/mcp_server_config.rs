//! HTTP MCP server configuration the Rust client passes to the Codex
//! runner over stdio. Codex's `Thread` API only consumes HTTP MCP
//! servers, so this is a single struct (no Stdio/SSE variants).
//!
//! The runner currently ignores `mcp_servers` — wiring this into
//! `Codex.Thread` is a follow-up. The wire shape is stable so callers
//! can start including it now.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// HTTP MCP server config — URL plus optional headers.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpServerConfig {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<IndexMap<String, String>>,
}

impl From<&objectiveai::mcp::Connection> for McpServerConfig {
    fn from(conn: &objectiveai::mcp::Connection) -> Self {
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
