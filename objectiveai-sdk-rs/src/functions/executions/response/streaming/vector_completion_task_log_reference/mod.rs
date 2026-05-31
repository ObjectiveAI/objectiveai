//! `LogReference` for a vector-completion task within a parent
//! function-execution log file. Carries the task's hierarchical
//! positional identity and an optional wrapper-level `error`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::filesystem::logs::LogReferenceTag;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "functions.executions.response.streaming.vector_completion_task_log_reference.LogReference")]
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
    pub error: Option<serde_json::Value>,
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
            error: None,
        }
    }
}
