//! The assistant reasoning chunk.

use rmcp::model::TextContent;
use serde::{Deserialize, Serialize};

use super::Logprob;

/// The model's reasoning.
///
/// A DELTA, like [`AssistantTextContentChunk`](super::AssistantTextContentChunk):
/// fragments arrive and a caller concatenates them.
///
/// The payload is MCP's [`TextContent`], not a bare `String`. Same
/// vocabulary as ordinary content, so nothing here needs its own
/// handling — and `_meta` and `annotations` come along, which a
/// `String` has nowhere to put.
///
/// Structurally identical to the text and refusal chunks, and that is
/// fine precisely because the `type` constants differ: the untagged
/// enum decides on the discriminator, never on shape, so payloads may
/// coincide without becoming ambiguous.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssistantReasoningChunk {
    /// The discriminator. See [`AgenticLoopChunk`](super::AgenticLoopChunk).
    pub r#type: AssistantReasoningChunkType,
    /// The tool call whose sub-agent produced this chunk; absent on
    /// the main thread. A nested sub-agent names its IMMEDIATE
    /// spawning call, so depth is a chain of ids a caller can follow.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_tool_call_id: Option<String>,
    /// Per-token log probabilities for this fragment, when requested.
    ///
    /// Scoped to THIS chunk's tokens, not the turn's — each delta
    /// carries the probabilities for the text it delivers, so a caller
    /// that concatenates the text can concatenate these alongside it
    /// and keep them aligned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<Vec<Logprob>>,
    /// The reasoning itself.
    #[serde(flatten)]
    pub inner: TextContent,
}

/// [`AssistantReasoningChunk`]'s discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AssistantReasoningChunkType {
    #[serde(rename = "assistant_reasoning")]
    #[default]
    AssistantReasoning,
}

impl AssistantReasoningChunk {
    /// Merge the fragment that arrived directly behind this one: the
    /// text concatenates, the log probabilities append. What else the
    /// other fragment carried — its `_meta`, its annotations — is
    /// dropped in favour of this chunk's own; fragments of one run
    /// say the same things there.
    pub fn push(&mut self, other: Self) {
        self.inner.text.push_str(&other.inner.text);
        match (&mut self.logprobs, other.logprobs) {
            (Some(logprobs), Some(other)) => logprobs.extend(other),
            (None, Some(other)) => self.logprobs = Some(other),
            _ => {}
        }
    }
}
