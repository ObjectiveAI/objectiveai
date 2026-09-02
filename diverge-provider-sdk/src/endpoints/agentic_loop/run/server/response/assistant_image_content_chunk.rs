//! The assistant image content chunk.

use rmcp::model::ImageContent;
use serde::{Deserialize, Serialize};

/// An image from the model.
///
/// Whole, not a delta — the payload is base64 data with a MIME
/// type, and half of a base64 image is not an image.
///
/// The payload is MCP's own [`ImageContent`], flattened, so the
/// content the model produced is expressed in the same vocabulary a
/// tool would use to return it — one content model across the whole
/// loop rather than one per direction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssistantImageContentChunk {
    /// The discriminator. See [`ContinuationChunk`](super::ContinuationChunk).
    pub r#type: AssistantImageContentChunkType,
    /// The tool call whose sub-agent produced this chunk; absent on
    /// the main thread. A nested sub-agent names its IMMEDIATE
    /// spawning call, so depth is a chain of ids a caller can follow.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_tool_call_id: Option<String>,
    /// The content itself.
    #[serde(flatten)]
    pub inner: ImageContent,
}

/// [`AssistantImageContentChunk`]'s discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AssistantImageContentChunkType {
    #[serde(rename = "assistant_image_content")]
    #[default]
    AssistantImageContent,
}
