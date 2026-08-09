//! The assistant refusal chunk.

use rmcp::model::TextContent;
use serde::{Deserialize, Serialize};

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
