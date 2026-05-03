//! Caller-supplied additional MCP server for an agent-completion run.
//!
//! `extra_mcp_servers` lets a caller (e.g. the function-inventions
//! orchestrator) attach extra MCP upstreams onto a single
//! `create_streaming` call without baking those URLs into the agent's
//! own content-hashed configuration. Each entry pairs a URL with an
//! optional per-server header set; both flow through to the proxy as
//! one URL in `X-MCP-Servers` and one entry in `X-MCP-Headers`'s
//! per-URL header map (merged on top of any orchestrator-wide
//! `extra_mcp_headers`, with the per-server values winning on conflict).

use indexmap::IndexMap;

/// One extra MCP upstream to attach to an agent-completion request.
#[derive(Debug, Clone)]
pub struct ExtraMcpServer {
    /// The upstream URL the proxy should connect to.
    pub url: String,
    /// Per-server HTTP headers stamped on every request the proxy
    /// makes to this upstream. `Authorization`, custom `X-*`,
    /// anything. `None` is equivalent to an empty map.
    pub headers: Option<IndexMap<String, String>>,
}

impl ExtraMcpServer {
    /// Convenience constructor with no per-server headers.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            headers: None,
        }
    }
}
