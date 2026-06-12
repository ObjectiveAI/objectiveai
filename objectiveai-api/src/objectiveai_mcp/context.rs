//! Shared per-request context handed to every MCP delegate.

use super::ReverseChannelRegistry;
use axum::http::HeaderMap;

/// Shared state every MCP delegate receives. Built once per HTTP
/// request by the axum handler before calling the delegate; cheap
/// to construct (every field is `Clone` of an `Arc`-backed handle).
///
/// The GET-SSE handler reads its listener registry directly from the
/// shared router state, so this ctx carries only what the JSON-RPC
/// (POST) and DELETE delegates need.
pub struct McpRequestContext {
    /// Routing key — value of the `X-OBJECTIVEAI-RESPONSE-ID` header
    /// on the inbound request. Looked up in `registry` to find the
    /// matching WS reverse channel. Distinct from `Mcp-Session-Id`
    /// (which rides through `headers` to whatever upstream MCP server
    /// sits behind the CLI's conduit).
    pub response_id: String,
    /// Verbatim request headers. Forwarded through to the CLI's
    /// upstream MCP (modulo a hop-by-hop blacklist) by every
    /// forwarding delegate. `Mcp-Session-Id` rides through here.
    pub headers: HeaderMap,
    /// Reverse-channel registry. Delegates look up their target WS
    /// by `response_id` and call [`super::send_server_request`].
    pub registry: ReverseChannelRegistry,
    /// Budget for one WS reverse-channel round-trip (from
    /// `Config.reverse_channel_timeout`).
    pub reverse_channel_timeout: std::time::Duration,
}
