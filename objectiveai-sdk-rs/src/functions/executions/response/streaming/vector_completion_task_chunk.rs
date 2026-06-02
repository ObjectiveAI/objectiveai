use crate::agent::completions::response::streaming::AgentCompletionIds;
use crate::{error, vector};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    Default,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(
    rename = "functions.executions.response.streaming.VectorCompletionTaskChunk"
)]
pub struct VectorCompletionTaskChunk {
    #[arbitrary(with = crate::arbitrary_util::arbitrary_u64)]
    pub index: u64,
    #[arbitrary(with = crate::arbitrary_util::arbitrary_u64)]
    pub task_index: u64,
    #[arbitrary(with = crate::arbitrary_util::arbitrary_vec_u64)]
    pub task_path: Vec<u64>,
    #[serde(flatten)]
    pub inner: vector::completions::response::streaming::VectorCompletionChunk,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub error: Option<error::ResponseError>,
}

impl AgentCompletionIds for VectorCompletionTaskChunk {
    fn agent_completion_ids(&self) -> impl Iterator<Item = &str> {
        self.inner.agent_completion_ids()
    }
}

impl VectorCompletionTaskChunk {
    pub fn push(&mut self, other: &VectorCompletionTaskChunk) {
        self.inner.push(&other.inner);
        if let Some(error) = &other.error {
            self.error = Some(error.clone());
        }
    }

    /// Delegates to the inner vector completion.
    #[cfg(feature = "filesystem")]
    pub fn produce_message_rows(
        &self,
    ) -> impl Iterator<Item = crate::filesystem::db::schema::MessageRow> + Send + '_
    {
        self.inner.produce_message_rows()
    }
}
