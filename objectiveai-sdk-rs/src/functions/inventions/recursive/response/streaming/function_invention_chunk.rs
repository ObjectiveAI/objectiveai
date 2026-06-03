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
    rename = "functions.inventions.recursive.response.streaming.FunctionInventionChunk"
)]
pub struct FunctionInventionChunk {
    #[arbitrary(with = crate::arbitrary_util::arbitrary_u64)]
    pub index: u64,
    #[serde(flatten)]
    pub inner:
        functions::inventions::response::streaming::FunctionInventionChunk,
}

impl AgentCompletionIds for FunctionInventionChunk {
    fn agent_completion_ids(&self) -> impl Iterator<Item = &str> + Send {
        self.inner.agent_completion_ids()
    }
}

impl FunctionInventionChunk {
    pub fn push(&mut self, other: &FunctionInventionChunk) {
        self.inner.push(&other.inner);
    }

}
