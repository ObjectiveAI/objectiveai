use crate::functions::executions::response;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Task {
    FunctionExecution(super::FunctionExecutionTask),
    VectorCompletion(super::VectorCompletionTask),
}

impl Task {
    pub fn task_path(&self) -> &Vec<u64> {
        match self {
            Task::FunctionExecution(f) => &f.task_path,
            Task::VectorCompletion(v) => &v.task_path,
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
