//! Streaming chat completion chunks from OpenRouter.

use std::collections::HashMap;

use diverge_provider_sdk::endpoints::containers::agents::run::server::response;

use serde::Deserialize;

/// A streaming chat completion chunk from OpenRouter.
///
/// Contains partial response data that arrives incrementally during streaming.
#[derive(Debug, Clone, Deserialize, Default)]
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
    pub service_tier: Option<String>,
    /// System fingerprint for reproducibility.
    pub system_fingerprint: Option<String>,
    /// Token usage statistics (typically in the final chunk).
    pub usage: Option<super::Usage>,
    /// The upstream provider that served this request.
    pub provider: Option<String>,
}

impl ChatCompletionChunk {
    /// Append this upstream chunk's contribution to the loop's chunk
    /// stream, in the fixed order: reasoning, content, images,
    /// refusal, tool calls — then usage.
    ///
    /// Every chunk appended here is stamped with an `openrouter`
    /// entry in its `_meta` — `{"id": <the completion's id>}` — the
    /// provenance a chunk keeps when it leaves the stream that gave
    /// it context.
    ///
    /// `tool_calls` carries call identity across fragments: OpenRouter
    /// sends a call's `id` and `name` only in its first fragment, so a
    /// fragment that has them enters the map by its `index`, and one
    /// that does not reads the map. See
    /// [`AssistantToolCallDelta::into_chunks`](crate::request::AssistantToolCallDelta::into_chunks).
    ///
    /// Only the FIRST choice is read: an OpenRouter completion carries
    /// exactly one, and anything beyond it is dropped. The rest of the
    /// envelope — `created`, `model`, `object`, `service_tier`,
    /// `system_fingerprint`, `provider` — describes the stream rather
    /// than anything a chunk carries, and is dropped with it.
    pub fn into_chunks(
        self,
        tool_calls: &mut HashMap<u64, (String, String)>,
        chunks: &mut Vec<response::AgenticLoopChunk>,
    ) {
        let openrouter = serde_json::json!({
            "id": self.id,
        });

        let start = chunks.len();
        if let Some(choice) = self.choices.into_iter().next() {
            choice.into_chunks(tool_calls, chunks);
        }
        if let Some(usage) = self.usage {
            usage.into_chunks(chunks);
        }
        for chunk in &mut chunks[start..] {
            stamp(chunk, &openrouter);
        }
    }
}

/// Insert the `openrouter` provenance entry into one chunk's `_meta`.
///
/// Every chunk kind has a `_meta` slot — the content-bearing kinds
/// through the MCP content they flatten, the rest through their own
/// `meta` field — created here when absent.
fn stamp(
    chunk: &mut response::AgenticLoopChunk,
    openrouter: &serde_json::Value,
) {
    use response::AgenticLoopChunk;

    let map: &mut rmcp::model::JsonObject = match chunk {
        AgenticLoopChunk::AssistantReasoning(chunk) => {
            &mut chunk.inner.meta.get_or_insert_with(Default::default).0
        }
        AgenticLoopChunk::AssistantTextContent(chunk) => {
            &mut chunk.inner.meta.get_or_insert_with(Default::default).0
        }
        AgenticLoopChunk::AssistantImageContent(chunk) => {
            &mut chunk.inner.meta.get_or_insert_with(Default::default).0
        }
        AgenticLoopChunk::AssistantAudioContent(chunk) => {
            &mut chunk.inner.meta.get_or_insert_with(Default::default).0
        }
        AgenticLoopChunk::AssistantRefusal(chunk) => {
            &mut chunk.inner.meta.get_or_insert_with(Default::default).0
        }
        AgenticLoopChunk::AssistantToolCall(chunk) => {
            &mut chunk.meta.get_or_insert_with(Default::default).0.0
        }
        AgenticLoopChunk::ToolResponse(chunk) => {
            &mut chunk.inner.meta.get_or_insert_with(Default::default).0
        }
        AgenticLoopChunk::Usage(chunk) => {
            &mut chunk.meta.get_or_insert_with(Default::default).0
        }
        AgenticLoopChunk::Notification(chunk) => {
            &mut chunk.meta.get_or_insert_with(Default::default).0
        }
        AgenticLoopChunk::UserTextContent(chunk) => {
            &mut chunk.inner.meta.get_or_insert_with(Default::default).0
        }
        AgenticLoopChunk::UserImageContent(chunk) => {
            &mut chunk.inner.meta.get_or_insert_with(Default::default).0
        }
        AgenticLoopChunk::UserAudioContent(chunk) => {
            &mut chunk.inner.meta.get_or_insert_with(Default::default).0
        }
        AgenticLoopChunk::UserResource(chunk) => {
            &mut chunk.inner.meta.get_or_insert_with(Default::default).0
        }
        AgenticLoopChunk::UserResourceLink(chunk) => {
            &mut chunk.inner.meta.get_or_insert_with(Default::default).0
        }
    };
    map.insert("openrouter".to_string(), openrouter.clone());
}
