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
    /// The request messages the vector client dispatched to this
    /// agent. Populated ONLY on the FIRST chunk of the completion;
    /// [`push`](Self::push) keeps the first value.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[arbitrary(default)]
    pub request_messages: Option<Vec<crate::agent::completions::message::Message>>,
    /// The resolved inline WF definition for this agent. Populated
    /// ONLY on the FIRST chunk of the completion; [`push`](Self::push)
    /// keeps the first value.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[arbitrary(default)]
    pub agent_inline: Option<crate::agent::InlineAgentWithFallbacks>,
    /// The underlying agent completion chunk.
    #[serde(flatten)]
    pub inner: agent::completions::response::streaming::AgentCompletionChunk,
}

impl AgentCompletionIds for AgentCompletionChunk {
    fn agent_completion_ids(&self) -> impl Iterator<Item = &str> + Send {
        self.inner.agent_completion_ids()
    }
}

impl AgentCompletionChunk {
    pub fn push(&mut self, other: &AgentCompletionChunk) {
        self.inner.push(&other.inner);
        // First chunk wins: `request_messages` and `agent_inline` ride
        // only the completion's first chunk, so the accumulator never
        // overwrites them.
        if self.request_messages.is_none() {
            if let Some(request_messages) = &other.request_messages {
                self.request_messages = Some(request_messages.clone());
            }
        }
        if self.agent_inline.is_none() {
            if let Some(agent_inline) = &other.agent_inline {
                self.agent_inline = Some(agent_inline.clone());
            }
        }
    }
}
