use crate::functions;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, arbitrary::Arbitrary)]
#[schemars(rename = "functions.profiles.computations.response.streaming.FunctionExecutionChunk")]
pub struct FunctionExecutionChunk {
    #[arbitrary(with = crate::arbitrary_util::arbitrary_u64)]
    pub index: u64,
    #[arbitrary(with = crate::arbitrary_util::arbitrary_u64)]
    pub dataset: u64,
    #[arbitrary(with = crate::arbitrary_util::arbitrary_u64)]
    pub n: u64,
    #[arbitrary(with = crate::arbitrary_util::arbitrary_u64)]
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
