use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "functions.executions.request.Reasoning")]
pub struct Reasoning {
    pub agent: crate::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional,
}
