use crate::functions::executions::response;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionExecutionTask {
    pub index: u64,
    pub task_index: u64,
    pub task_path: Vec<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub swiss_pool_index: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub swiss_round: Option<u64>,
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
            inner,
        }: response::streaming::FunctionExecutionTaskChunk,
    ) -> Self {
        Self {
            index,
            task_index,
            task_path,
            swiss_pool_index,
            swiss_round,
            inner: inner.into(),
        }
    }
}
