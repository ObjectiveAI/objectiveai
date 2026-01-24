use crate::functions::{self, profiles::computations::response};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionExecution {
    pub index: u64,
    pub dataset: u64,
    pub n: u64,
    pub retry: u64,
    #[serde(flatten)]
    pub inner: functions::executions::response::unary::FunctionExecution,
}

impl From<response::streaming::FunctionExecutionChunk> for FunctionExecution {
    fn from(
        response::streaming::FunctionExecutionChunk {
            index,
            dataset,
            n,
            retry,
            inner,
        }: response::streaming::FunctionExecutionChunk,
    ) -> Self {
        Self {
            index,
            dataset,
            n,
            retry,
            inner: inner.into(),
        }
    }
}
