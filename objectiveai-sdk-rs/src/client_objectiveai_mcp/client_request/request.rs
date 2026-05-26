use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Envelope: correlation `id` + tagged [`super::Payload`]. Wire shape
/// (the `id` field lives at the envelope level, the `type` discriminator
/// and the variant's payload fields are flattened alongside):
///
/// ```json
/// {"id":"…","type":"agent_completion_notify","response_id":"…","content":{…}}
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.client_request.Request")]
pub struct Request {
    /// Client-minted correlation id. Echoed by the matching
    /// [`super::super::client_response::Response`].
    pub id: String,
    #[serde(flatten)]
    pub payload: super::Payload,
}
