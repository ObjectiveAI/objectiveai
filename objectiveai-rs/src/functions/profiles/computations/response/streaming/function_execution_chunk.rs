use crate::functions;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionExecutionChunk {
    pub index: u64,
    pub dataset: u64,
    pub n: u64,
    pub retry: u64,
    #[serde(flatten)]
    pub inner:
        functions::executions::response::streaming::FunctionExecutionChunk,
}

impl FunctionExecutionChunk {
    pub fn push(&mut self, other: &FunctionExecutionChunk) {
        self.inner.push(&other.inner);
    }
}
