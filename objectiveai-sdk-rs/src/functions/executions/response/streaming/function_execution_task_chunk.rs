use crate::agent::completions::response::streaming::AgentCompletionIds;
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
    rename = "functions.executions.response.streaming.FunctionExecutionTaskChunk"
)]
pub struct FunctionExecutionTaskChunk {
    #[arbitrary(with = crate::arbitrary_util::arbitrary_u64)]
    pub index: u64,
    #[arbitrary(with = crate::arbitrary_util::arbitrary_u64)]
    pub task_index: u64,
    #[arbitrary(with = crate::arbitrary_util::arbitrary_vec_u64)]
    pub task_path: Vec<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_option_u64)]
    pub swiss_pool_index: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_option_u64)]
    pub swiss_round: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_option_u64)]
    pub split_index: Option<u64>,
    #[serde(flatten)]
    pub inner: super::FunctionExecutionChunk,
}

impl AgentCompletionIds for FunctionExecutionTaskChunk {
    fn agent_completion_ids(&self) -> impl Iterator<Item = &str> + Send {
        self.inner.agent_completion_ids()
    }
}

impl FunctionExecutionTaskChunk {
    pub fn push(&mut self, other: &super::FunctionExecutionTaskChunk) {
        self.inner.push(&other.inner);
    }

}
