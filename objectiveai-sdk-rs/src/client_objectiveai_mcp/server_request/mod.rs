//! Requests addressed to the MCP-server protocol layer of the local
//! objectiveai-mcp — the API forwards the proxy's HTTP request to
//! `/objectiveai-mcp/{session_id}` verbatim (method + headers + body)
//! and the calling client's `McpHandler` returns a plain HTTP
//! response. The CLI's `ConduitMcpHandler` is the canonical
//! implementation: it spawns `objectiveai-mcp` as a subprocess and
//! pipes requests through.
//!
//! Each request carries a server-minted `id` that the client echoes
//! in the matching [`super::server_response::Response`] so the
//! server can correlate replies to in-flight requests.

mod request;
pub use request::*;
mod payload;
pub use payload::*;
