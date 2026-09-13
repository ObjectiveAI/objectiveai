//! Delta type for streaming responses.

use std::collections::HashMap;

use diverge_provider_sdk::endpoints::containers::agents::run::server::response;

use serde::Deserialize;

/// A delta (incremental update) in a streaming response.
///
/// Each field contains only the new content since the last delta;
/// accumulation happens downstream, in the SDK's own chunk vocabulary.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct Delta {
    /// New content text since the last delta.
    pub content: Option<String>,
    /// New refusal text since the last delta.
    pub refusal: Option<String>,
    /// The role (only present in the first delta).
    pub role: Option<super::Role>,
    /// Tool call updates.
    pub tool_calls:
        Option<Vec<crate::request::AssistantToolCallDelta>>,

    /// New reasoning text since the last delta.
    pub reasoning: Option<String>,
    /// New generated images.
    pub images: Option<Vec<super::Image>>,
}

impl Delta {
    /// Append this delta's chunks, in the fixed order: reasoning,
    /// content, images, refusal, tool calls. `logprobs` are the
    /// choice's, split to the chunks they describe — content's to the
    /// text chunk, refusal's to the refusal chunk. The `role` says
    /// nothing a chunk carries and is dropped.
    pub fn into_chunks(
        self,
        logprobs: Option<super::Logprobs>,
        tool_calls: &mut HashMap<u64, (String, String)>,
        chunks: &mut Vec<response::AgenticLoopChunk>,
    ) {
        let (content_logprobs, refusal_logprobs) = match logprobs {
            Some(logprobs) => (logprobs.content, logprobs.refusal),
            None => (None, None),
        };

        if let Some(reasoning) = self.reasoning {
            chunks.push(response::AgenticLoopChunk::AssistantReasoning(
                response::AssistantReasoningChunk {
                    r#type: Default::default(),
                    parent_tool_call_id: None,
                    logprobs: None,
                    inner: rmcp::model::TextContent::new(reasoning),
                },
            ));
        }
        if let Some(content) = self.content {
            chunks.push(response::AgenticLoopChunk::AssistantTextContent(
                response::AssistantTextContentChunk {
                    r#type: Default::default(),
                    parent_tool_call_id: None,
                    logprobs: super::logprobs::into_chunk_logprobs(content_logprobs),
                    inner: rmcp::model::TextContent::new(content),
                },
            ));
        }
        for image in self.images.into_iter().flatten() {
            image.into_chunks(chunks);
        }
        if let Some(refusal) = self.refusal {
            chunks.push(response::AgenticLoopChunk::AssistantRefusal(
                response::AssistantRefusalChunk {
                    r#type: Default::default(),
                    parent_tool_call_id: None,
                    logprobs: super::logprobs::into_chunk_logprobs(refusal_logprobs),
                    inner: rmcp::model::TextContent::new(refusal),
                },
            ));
        }
        for tool_call in self.tool_calls.into_iter().flatten() {
            tool_call.into_chunks(tool_calls, chunks);
        }
    }
}
