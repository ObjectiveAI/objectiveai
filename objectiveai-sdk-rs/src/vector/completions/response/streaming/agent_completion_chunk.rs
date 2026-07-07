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
    /// The resolved inline WF definition for this agent. Populated
    /// ONLY on the FIRST chunk of the completion; [`push`](Self::push)
    /// keeps the first value.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[arbitrary(default)]
    pub agent_inline: Option<crate::agent::InlineAgentWithFallbacks>,
    /// This agent's prefix-tree voting key for each response choice, in
    /// the SAME order as the request's `responses`
    /// (`request_choice_keys[i]` is this agent's key for choice `i`).
    /// Keys are per-agent (randomized), so they differ between agents.
    /// Populated ONLY on the FIRST chunk of the completion;
    /// [`push`](Self::push) keeps the first value.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[arbitrary(default)]
    pub request_choice_keys: Option<Vec<String>>,
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
        // First chunk wins: `agent_inline` and `request_choice_keys`
        // ride only the completion's first chunk, so the accumulator
        // never overwrites them.
        if self.agent_inline.is_none() {
            if let Some(agent_inline) = &other.agent_inline {
                self.agent_inline = Some(agent_inline.clone());
            }
        }
        if self.request_choice_keys.is_none() {
            if let Some(request_choice_keys) = &other.request_choice_keys {
                self.request_choice_keys = Some(request_choice_keys.clone());
            }
        }
    }
}
