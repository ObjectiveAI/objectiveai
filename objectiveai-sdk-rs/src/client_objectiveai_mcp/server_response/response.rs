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
    /// The typed response variant. The MCP-routed variants echo
    /// `mcp_kind` inside the variant itself (see [`super::Payload`]);
    /// non-MCP variants don't.
    #[serde(flatten)]
    pub payload: super::Payload,
}

impl Response {
    /// Which CLI-hosted MCP server this response came from. `Some` for
    /// the MCP-routed variants; `None` for non-MCP variants. Delegates
    /// to [`super::Payload::mcp_kind`].
    pub fn mcp_kind(&self) -> Option<super::super::McpKind> {
        self.payload.mcp_kind()
    }
}
