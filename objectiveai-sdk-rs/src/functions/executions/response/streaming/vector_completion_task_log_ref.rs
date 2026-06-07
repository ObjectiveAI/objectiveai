//! `VectorCompletionTaskLogRef` — ref into
//! `logs.vector_completion_responses` for a vector-completion task
//! inside a function-execution chunk. Includes the wrapper-level
//! `error` so a task that failed before producing an id-bearing
//! chunk still has its failure visible at the parent's ref level.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error;
use crate::logs::LogRef;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(
    rename = "functions.executions.response.streaming.VectorCompletionTaskLogRef"
)]
pub struct VectorCompletionTaskLogRef {
    #[serde(flatten)]
    pub log_ref: LogRef,
    pub index: u64,
    pub task_index: u64,
    pub task_path: Vec<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub error: Option<error::ResponseError>,
}
