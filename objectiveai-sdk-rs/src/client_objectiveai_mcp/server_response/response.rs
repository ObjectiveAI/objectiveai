use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// One HTTP response the calling client's `McpHandler` produces in
/// reply to a [`super::super::server_request::Request`]. The API
/// translates this into the HTTP response it returns to the proxy.
///
/// Wire shape:
///
/// ```json
/// {"id":"…","status":200,"headers":{"Content-Type":"application/json"},"body":{…}}
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.server_response.Response")]
pub struct Response {
    /// Matches the `id` of the
    /// [`super::super::server_request::Request`] this response is for.
    pub id: String,
    /// HTTP status code (`200` for success, `4xx` / `5xx` for errors).
    pub status: u16,
    /// Response headers. Typically just `Content-Type` and possibly
    /// `Mcp-Session-Id` on the `initialize` response.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    #[schemars(extend("omitempty" = true))]
    pub headers: IndexMap<String, String>,
    /// Response body — the JSON-RPC reply envelope. `None` for empty
    /// responses (e.g. `202 Accepted` on notifications).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub body: Option<serde_json::Value>,
}
