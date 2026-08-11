//! The assistant refusal chunk.

use rmcp::model::TextContent;
use serde::{Deserialize, Serialize};

use super::Logprob;

/// The model declining to answer.
///
/// A DELTA, like [`AssistantTextContentChunk`](super::AssistantTextContentChunk):
/// fragments arrive and a caller concatenates them.
///
/// The payload is MCP's [`TextContent`], not a bare `String`. Same
/// vocabulary as ordinary content, so nothing here needs its own
/// handling — and `_meta` and `annotations` come along, which a
/// `String` has nowhere to put.
///
/// Structurally identical to the text and reasoning chunks, and that is
/// fine precisely because the `type` constants differ: the untagged
/// enum decides on the discriminator, never on shape, so payloads may
/// coincide without becoming ambiguous.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssistantRefusalChunk {
    /// The discriminator. See [`ContinuationChunk`](super::ContinuationChunk).
    pub r#type: AssistantRefusalChunkType,
    /// Per-token log probabilities for this fragment, when requested.
    ///
    /// Scoped to THIS chunk's tokens, not the turn's — each delta
    /// carries the probabilities for the text it delivers, so a caller
    /// that concatenates the text can concatenate these alongside it
    /// and keep them aligned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<Vec<Logprob>>,
    /// The refusal itself.
    #[serde(flatten)]
    pub inner: TextContent,
}

/// [`AssistantRefusalChunk`]'s discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AssistantRefusalChunkType {
    #[serde(rename = "assistant_refusal")]
    #[default]
    AssistantRefusal,
}
