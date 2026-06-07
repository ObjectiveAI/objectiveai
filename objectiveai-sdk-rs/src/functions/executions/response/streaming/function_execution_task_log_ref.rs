//! `FunctionExecutionTaskLogRef` — ref into
//! `logs.function_execution_responses` for a nested function task.
//! Carries the task's hierarchical positional identity and optional
//! swiss / split metadata.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::logs::LogRef;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(
    rename = "functions.executions.response.streaming.FunctionExecutionTaskLogRef"
)]
pub struct FunctionExecutionTaskLogRef {
    #[serde(flatten)]
    pub log_ref: LogRef,
    pub index: u64,
    pub task_index: u64,
    pub task_path: Vec<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub swiss_pool_index: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub swiss_round: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub split_index: Option<u64>,
}
