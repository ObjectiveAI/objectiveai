//! `LogReference` for a nested function-execution task within a
//! parent function-execution log file. Carries the task's full
//! hierarchical positional identity (`index`, `task_index`,
//! `task_path`) plus optional swiss-system / split-iteration
//! per-iteration ids.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::LogReferenceTag;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(
    rename = "functions.executions.response.streaming.function_execution_task_log_reference.LogReference"
)]
pub struct LogReference {
    #[serde(rename = "type")]
    pub r#type: LogReferenceTag,
    #[serde(skip_serializing_if = "String::is_empty")]
    #[schemars(extend("omitempty" = true))]
    pub path: String,
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

impl LogReference {
    pub fn new(
        path: String,
        index: u64,
        task_index: u64,
        task_path: Vec<u64>,
    ) -> Self {
        Self {
            r#type: LogReferenceTag::Reference,
            path,
            index,
            task_index,
            task_path,
            swiss_pool_index: None,
            swiss_round: None,
            split_index: None,
        }
    }
}
