use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionExecutionTaskChunk {
    pub index: u64,
    pub task_index: u64,
    pub task_path: Vec<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub swiss_pool_index: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub swiss_round: Option<u64>,
    #[serde(flatten)]
    pub inner: super::FunctionExecutionChunk,
}

impl FunctionExecutionTaskChunk {
    pub fn push(&mut self, other: &super::FunctionExecutionTaskChunk) {
        self.inner.push(&other.inner);
    }
}
