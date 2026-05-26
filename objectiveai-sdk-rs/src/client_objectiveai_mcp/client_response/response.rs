use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Reply to a [`super::super::client_request::Request`]. Wire shape:
///
/// ```json
/// {"id":"…","type":"ok"}                              // success
/// {"id":"…","type":"error","code":404,"message":…}    // failure
/// ```
///
/// The internally-tagged enum keeps the wire flat — `id`, `type`,
/// and any extra fields sit side-by-side on the JSON object — while
/// keeping the schema clean (no `anyOf`-with-sibling-properties
/// structural quirk that flattening an inner Result enum would
/// otherwise produce).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.client_response.Response")]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Response {
    /// Empty success — the request was accepted.
    #[schemars(title = "client_objectiveai_mcp.client_response.Response.Ok")]
    Ok {
        /// Matches the `id` of the
        /// [`super::super::client_request::Request`] this response is for.
        id: String,
    },
    /// The request failed.
    #[schemars(title = "client_objectiveai_mcp.client_response.Response.Error")]
    Error {
        /// Matches the `id` of the
        /// [`super::super::client_request::Request`] this response is for.
        id: String,
        code: u16,
        message: serde_json::Value,
    },
}

impl Response {
    /// The correlation id present on every variant.
    pub fn id(&self) -> &str {
        match self {
            Response::Ok { id } => id,
            Response::Error { id, .. } => id,
        }
    }
}
