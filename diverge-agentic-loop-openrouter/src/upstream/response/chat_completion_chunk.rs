//! Streaming chat completion chunks from OpenRouter.

use std::collections::HashMap;

use diverge_provider_sdk::endpoints::agentic_loop::run::server::response;

use serde::{Deserialize, Serialize};

/// A streaming chat completion chunk from OpenRouter.
///
/// Contains partial response data that arrives incrementally during streaming.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChatCompletionChunk {
    /// Unique identifier for this completion from OpenRouter.
    pub id: String,
    /// Completion choices containing the generated content.
    pub choices: Vec<super::Choice>,
    /// Unix timestamp when the completion was created.
    pub created: u64,
    /// The model that generated this completion.
    pub model: String,
    /// Object type indicator.
    pub object: super::Object,
    /// The service tier used for this request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<String>,
    /// System fingerprint for reproducibility.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<String>,
    /// Token usage statistics (typically in the final chunk).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<super::Usage>,
    /// The upstream provider that served this request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
}

impl ChatCompletionChunk {
    /// Append this upstream chunk's contribution to the loop's chunk
    /// stream, in the fixed order: reasoning, content, images,
    /// refusal, tool calls — then usage.
    ///
    /// `tool_calls` carries call identity across fragments: OpenRouter
    /// sends a call's `id` and `name` only in its first fragment, so a
    /// fragment that has them enters the map by its `index`, and one
    /// that does not reads the map. See
    /// [`AssistantToolCallDelta::into_chunks`](crate::upstream::request::AssistantToolCallDelta::into_chunks).
    ///
    /// Only the FIRST choice is read: an OpenRouter completion carries
    /// exactly one, and anything beyond it is dropped. The envelope —
    /// `id`, `created`, `model`, `object`, `service_tier`,
    /// `system_fingerprint`, `provider` — describes the stream rather
    /// than anything a chunk carries, and is dropped with it.
    pub fn into_chunks(
        self,
        tool_calls: &mut HashMap<u64, (String, String)>,
        chunks: &mut Vec<response::AgenticLoopChunk>,
    ) {
        if let Some(choice) = self.choices.into_iter().next() {
            choice.into_chunks(tool_calls, chunks);
        }
        if let Some(usage) = self.usage {
            usage.into_chunks(chunks);
        }
    }
}
