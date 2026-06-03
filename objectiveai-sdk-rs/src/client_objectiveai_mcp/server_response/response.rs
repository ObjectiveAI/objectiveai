use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// One reply the calling client's `McpHandler` produced for a
/// [`super::super::server_request::Request`]. The typed
/// [`super::Payload`] variant pairs by name with the request side;
/// every method-specific result or error rides inside it.
///
/// Wire shape (envelope is `{id, type, …variant fields…}` after the
/// `#[serde(flatten)]` on `payload`):
///
/// ```json
/// {"id":"…","type":"tools_list","kind":"ok","result":{…}}
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.server_response.Response")]
pub struct Response {
    /// Matches the `id` of the
    /// [`super::super::server_request::Request`] this response is for.
    pub id: String,
    /// The typed response variant.
    #[serde(flatten)]
    pub payload: super::Payload,
}
