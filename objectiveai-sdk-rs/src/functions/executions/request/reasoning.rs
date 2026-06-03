use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "functions.executions.request.Reasoning")]
pub struct Reasoning {
    pub agent: crate::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional,
}
