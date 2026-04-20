use crate::functions::executions::response;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "functions.executions.response.unary.FunctionExecutionTask")]
pub struct FunctionExecutionTask {
    pub index: u64,
    pub task_index: u64,
    pub task_path: Vec<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub swiss_pool_index: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub swiss_round: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub split_index: Option<u64>,
    #[serde(flatten)]
    pub inner: super::FunctionExecution,
}

impl From<response::streaming::FunctionExecutionTaskChunk>
    for FunctionExecutionTask
{
    fn from(
        response::streaming::FunctionExecutionTaskChunk {
            index,
            task_index,
            task_path,
            swiss_pool_index,
            swiss_round,
            split_index,
            inner,
        }: response::streaming::FunctionExecutionTaskChunk,
    ) -> Self {
        Self {
            index,
            task_index,
            task_path,
            swiss_pool_index,
            swiss_round,
            split_index,
            inner: inner.into(),
        }
    }
}
