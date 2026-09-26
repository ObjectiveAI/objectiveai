//! Choice type for streaming responses.

use std::collections::HashMap;

use diverge_sdk::provider::endpoints::containers::agents::run::server::response;

use serde::Deserialize;

/// A choice in a streaming agent completion chunk.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct Choice {
    /// The content delta for this choice.
    pub delta: super::Delta,
    /// The reason generation stopped, if complete.
    pub finish_reason: Option<super::FinishReason>,
    /// The index of this choice.
    pub index: u64,
    /// Log probabilities for tokens, if requested.
    pub logprobs: Option<super::Logprobs>,
}

impl Choice {
    /// Append this choice's chunks: the delta's, with the choice's
    /// log probabilities beside them. The finish reason is a fact
    /// about the stream, not a chunk, and is dropped here.
    pub fn into_chunks(
        self,
        tool_calls: &mut HashMap<u64, (String, String)>,
        chunks: &mut Vec<response::AgenticLoopChunk>,
    ) {
        self.delta.into_chunks(self.logprobs, tool_calls, chunks);
    }
}
