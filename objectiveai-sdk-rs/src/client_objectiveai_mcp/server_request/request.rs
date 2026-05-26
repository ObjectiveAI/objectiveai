use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// One HTTP request the proxy made against the API's
/// `/objectiveai-mcp/{session_id}` route, forwarded verbatim down
/// the reverse-attach WS to the calling client. The client returns
/// a matching [`super::super::server_response::Response`] echoing
/// the request `id`.
///
/// Wire shape:
///
/// ```json
/// {"id":"…","method":"POST","headers":{"Mcp-Session-Id":"…"},"body":{…}}
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.server_request.Request")]
pub struct Request {
    /// Server-minted correlation id. Echoed by the matching
    /// [`super::super::server_response::Response`].
    pub id: String,
    /// HTTP method (`POST`, `GET`, `DELETE`).
    pub method: String,
    /// Verbatim copy of the headers the proxy sent on its HTTP
    /// request to the API. The handler stamps these on its own
    /// request to the local objectiveai-mcp, with `Accept` always
    /// forced to `application/json` so no SSE flows on the handler
    /// leg.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    #[schemars(extend("omitempty" = true))]
    pub headers: IndexMap<String, String>,
    /// Request body. `None` for `GET` and `DELETE`. For `POST`,
    /// this is the JSON-RPC envelope the proxy sent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub body: Option<serde_json::Value>,
}
