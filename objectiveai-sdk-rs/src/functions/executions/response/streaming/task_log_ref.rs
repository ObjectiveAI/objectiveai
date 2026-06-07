//! `TaskLogRef` — typed ref slot inside a function-execution chunk's
//! `tasks` array. Untagged on the wire — each variant's payload
//! carries a distinct field set.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{FunctionExecutionTaskLogRef, VectorCompletionTaskLogRef};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
#[schemars(
    rename = "functions.executions.response.streaming.TaskLogRef"
)]
pub enum TaskLogRef {
    #[schemars(title = "FunctionExecution")]
    FunctionExecution(FunctionExecutionTaskLogRef),
    #[schemars(title = "VectorCompletion")]
    VectorCompletion(VectorCompletionTaskLogRef),
}
