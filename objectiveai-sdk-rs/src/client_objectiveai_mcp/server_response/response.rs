use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// One reply the calling client's `McpHandler` produced for a
/// [`super::super::server_request::Request`]. The typed
/// [`super::Payload`] variant pairs by name with the request side;
/// every method-specific result or error rides inside it. `mcp_kind`
/// echoes the request's so the API can sanity-check the routing
/// without trusting `id` alone.
///
/// Wire shape (envelope is `{id, mcp_kind, type, …variant fields…}`
/// after the `#[serde(flatten)]` on `payload`):
///
/// ```json
/// {
///   "id":"…",
///   "mcp_kind":{"type":"objective_ai"},
///   "type":"tools_list",
///   "kind":"ok",
///   "result":{…}
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.server_response.Response")]
pub struct Response {
    /// Matches the `id` of the
    /// [`super::super::server_request::Request`] this response is for.
    pub id: String,
    /// Echoes the request envelope's `mcp_kind` so the API can
    /// confirm the CLI dispatched to the right per-MCP handler.
    pub mcp_kind: super::super::McpKind,
    /// The typed response variant.
    #[serde(flatten)]
    pub payload: super::Payload,
}
