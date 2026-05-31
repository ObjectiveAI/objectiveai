//! Untagged-enum `LogReference` for a task entry within a parent
//! function-execution log file. Dispatches between
//! [`super::function_execution_task_log_reference::LogReference`] and
//! [`super::vector_completion_task_log_reference::LogReference`]
//! depending on which task variant the parent's `TaskChunk`
//! produced.
//!
//! Serializes as the inner variant directly (no discriminator)
//! because the two task-reference shapes are already structurally
//! distinguishable on disk via their field sets.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "functions.executions.response.streaming.task_log_reference.LogReference")]
pub enum LogReference {
    #[schemars(title = "FunctionExecution")]
    FunctionExecution(super::function_execution_task_log_reference::LogReference),
    #[schemars(title = "VectorCompletion")]
    VectorCompletion(super::vector_completion_task_log_reference::LogReference),
}
