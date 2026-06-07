//! `AgentCompletionLogRef` — ref into `logs.agent_completion_responses`
//! for a per-agent slot inside a vector-completion chunk. Includes
//! the wrapper-level `error` so a slot that failed before producing
//! an id-bearing chunk still has its failure visible at the parent's
//! ref level.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error;
use crate::logs::LogRef;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(
    rename = "vector.completions.response.streaming.AgentCompletionLogRef"
)]
pub struct AgentCompletionLogRef {
    #[serde(flatten)]
    pub log_ref: LogRef,
    pub index: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub error: Option<error::ResponseError>,
}
