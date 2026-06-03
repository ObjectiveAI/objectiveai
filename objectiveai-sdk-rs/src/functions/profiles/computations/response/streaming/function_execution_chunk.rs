use crate::agent::completions::response::streaming::AgentCompletionIds;
use crate::functions;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(
    rename = "functions.profiles.computations.response.streaming.FunctionExecutionChunk"
)]
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

impl AgentCompletionIds for FunctionExecutionChunk {
    fn agent_completion_ids(&self) -> impl Iterator<Item = &str> + Send {
        self.inner.agent_completion_ids()
    }
}

impl FunctionExecutionChunk {
    pub fn push(&mut self, other: &FunctionExecutionChunk) {
        self.inner.push(&other.inner);
    }
}
