//! Streaming agent completion chunk for vector completions.

use crate::agent;
use crate::agent::completions::response::streaming::AgentCompletionIds;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A streaming agent completion chunk from a single agent within a vector completion.
///
/// The `index` field is used to correlate chunks belonging to the same
/// underlying completion when accumulating via [`push`](Self::push).
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
    rename = "vector.completions.response.streaming.AgentCompletionChunk"
)]
pub struct AgentCompletionChunk {
    /// Index used to correlate chunks from the same completion.
    #[arbitrary(with = crate::arbitrary_util::arbitrary_u64)]
    pub index: u64,
    /// The underlying agent completion chunk.
    #[serde(flatten)]
    pub inner: agent::completions::response::streaming::AgentCompletionChunk,
}

impl AgentCompletionIds for AgentCompletionChunk {
    fn agent_completion_ids(&self) -> impl Iterator<Item = &str> {
        self.inner.agent_completion_ids()
    }
}

impl AgentCompletionChunk {
    pub fn push(&mut self, other: &AgentCompletionChunk) {
        self.inner.push(&other.inner);
    }

}
