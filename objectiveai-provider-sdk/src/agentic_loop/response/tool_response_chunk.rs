//! The tool response chunk.

use serde::{Deserialize, Serialize};

use super::Content;

/// The result of one tool call.
///
/// Arrives whole, unlike the assistant chunks: a tool either returned
/// or it did not, so there is nothing to stream in pieces.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolResponseChunk {
    /// The discriminator. See [`ContinuationChunk`](super::ContinuationChunk).
    pub r#type: ToolResponseChunkType,
    /// The call this answers. The ONLY link back to it — results may
    /// arrive in a different order than the calls were made, so
    /// position in the stream proves nothing.
    pub tool_call_id: String,
    /// What the tool returned. Full [`Content`], not text, so a tool
    /// can hand back images and files rather than a description of
    /// them.
    pub content: Content,
    /// Vendor metadata, passed through untouched.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// [`ToolResponseChunk`]'s discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ToolResponseChunkType {
    #[serde(rename = "tool_response")]
    #[default]
    ToolResponse,
}
