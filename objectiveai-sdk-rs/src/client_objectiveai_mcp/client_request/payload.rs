use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Tagged union of request shapes the API can push down the
/// reverse-attach channel to the client-app layer.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.client_request.Payload")]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Payload {
    AgentCompletionNotify(crate::agent::completions::request::AgentCompletionNotifyParams),
}
