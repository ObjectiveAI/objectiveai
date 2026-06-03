use crate::functions::executions::response;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "functions.executions.response.unary.Task")]
pub enum Task {
    #[schemars(title = "FunctionExecution")]
    FunctionExecution(super::FunctionExecutionTask),
    #[schemars(title = "VectorCompletion")]
    VectorCompletion(super::VectorCompletionTask),
}

impl Task {
    pub fn task_path(&self) -> &Vec<u64> {
        match self {
            Task::FunctionExecution(f) => &f.task_path,
            Task::VectorCompletion(v) => &v.task_path,
        }
    }

    /// Returns a sort key that disambiguates concurrent siblings sharing
    /// a `task_path`. Pulls in the swiss / split / task indices that
    /// distinguish parallel branches so test snapshots are stable across
    /// arrival orders.
    pub fn snapshot_sort_key(
        &self,
    ) -> (Vec<u64>, Option<u64>, Option<u64>, Option<u64>, u64) {
        match self {
            Task::FunctionExecution(f) => (
                f.task_path.clone(),
                f.swiss_round,
                f.swiss_pool_index,
                f.split_index,
                f.task_index,
            ),
            Task::VectorCompletion(v) => {
                (v.task_path.clone(), None, None, None, v.task_index)
            }
        }
    }
}

impl From<response::streaming::TaskChunk> for Task {
    fn from(chunk: response::streaming::TaskChunk) -> Self {
        match chunk {
            response::streaming::TaskChunk::FunctionExecution(chunk) => {
                Task::FunctionExecution(chunk.into())
            }
            response::streaming::TaskChunk::VectorCompletion(chunk) => {
                Task::VectorCompletion(chunk.into())
            }
        }
    }
}
