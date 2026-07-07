//! Agent completion wrapper for vector completions.

use crate::{agent, vector::completions::response};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A agent completion from a single agent within a vector completion.
///
/// Wraps the standard agent completion response with an index to identify
/// which agent in the swarm produced it.
#[derive(Debug, Clone, Serialize, Deserialize, Default, JsonSchema)]
#[schemars(rename = "vector.completions.response.unary.AgentCompletion")]
pub struct AgentCompletion {
    /// Index of this completion within the vector completion.
    pub index: u64,
    /// The request messages the vector client dispatched to this
    /// agent, carried from the completion's first streaming chunk.
    pub request_messages: Vec<agent::completions::message::Message>,
    /// The underlying agent completion response.
    #[serde(flatten)]
    pub inner: agent::completions::response::unary::AgentCompletion,
}

impl From<response::streaming::AgentCompletionChunk> for AgentCompletion {
    fn from(
        response::streaming::AgentCompletionChunk {
            index,
            request_messages,
            inner,
        }: response::streaming::AgentCompletionChunk,
    ) -> Self {
        Self {
            index,
            request_messages: request_messages.unwrap_or_default(),
            inner: agent::completions::response::unary::AgentCompletion::from(
                inner,
            ),
        }
    }
}
