//! The assistant text content chunk.

use rmcp::model::TextContent;
use serde::{Deserialize, Serialize};

/// Text from the model.
///
/// A DELTA: text arrives in fragments, and a caller
/// concatenates them. Unlike the image and audio chunks, one of
/// these is rarely a whole anything.
///
/// The payload is MCP's own [`TextContent`], flattened, so the
/// content the model produced is expressed in the same vocabulary a
/// tool would use to return it — one content model across the whole
/// loop rather than one per direction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssistantTextContentChunk {
    /// The discriminator. See [`ContinuationChunk`](super::ContinuationChunk).
    pub r#type: AssistantTextContentChunkType,
    /// The content itself.
    #[serde(flatten)]
    pub inner: TextContent,
}

/// [`AssistantTextContentChunk`]'s discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AssistantTextContentChunkType {
    #[serde(rename = "assistant_text_content")]
    #[default]
    AssistantTextContent,
}
