//! HTTP MCP server configuration the Rust client passes to the Codex
//! runner over stdio. Codex's `Thread` API only consumes HTTP MCP
//! servers, so this is a single struct (no Stdio/SSE variants).
//!
//! Field names mirror the codex `config.toml` schema 1:1 (`url`,
//! `http_headers`) so the Python runner can serialize this map
//! straight into a `[mcp_servers.<name>]` table without a translation
//! step.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// HTTP MCP server config — URL plus optional HTTP headers. Mirrors
/// the codex `[mcp_servers.<name>]` TOML schema field-for-field.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpServerConfig {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_headers: Option<IndexMap<String, String>>,
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
            http_headers: if headers.is_empty() { None } else { Some(headers) },
        }
    }
}
