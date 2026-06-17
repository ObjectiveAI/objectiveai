//! ObjectiveAI MCP server — Streamable HTTP MCP, mounted under
//! `/objectiveai-mcp` with three routes (POST / GET / DELETE) and
//! routed by the `X-OBJECTIVEAI-RESPONSE-ID` header. Route surface
//! mirrors the MCP-spec subset of `objectiveai-mcp-proxy/src/mcp.rs`;
//! the proxy's ObjectiveAI-specific `/notify` extensions are not
//! mirrored.
//!
//! The reverse-attach plumbing the routes layer rides on top of —
//! [`registry`] (per-WS sink + pending-request slot, keyed by
//! per-agent `response_id`), [`listeners`] (per-MCP-session SSE broadcast
//! feeding the GET handler), [`send::send_server_request`] (the
//! API-side write primitive forwarding requests over the WS), and
//! [`sse::handle_get_sse`] — lives here too. Used to be in the SDK's
//! `mcp::conduit::server`; canonical home is now this module.

mod listeners;
mod registry;
mod send;

pub use listeners::McpListenerRegistry;
pub use registry::{
    PendingRequests, ReverseAttachConfig, ReverseAttachGuard, ReverseAttachHandle,
    ReverseChannel, ReverseChannelRegistry, SessionTracker, SharedSink,
    new_pending_requests, new_reverse_channel_registry,
};
pub use send::send_server_request;
