//! The assistant audio content chunk.

use rmcp::model::AudioContent;
use serde::{Deserialize, Serialize};

/// Audio from the model.
///
/// Whole, not a delta — as with images, the payload is base64
/// data with a MIME type.
///
/// The payload is MCP's own [`AudioContent`], flattened, so the
/// content the model produced is expressed in the same vocabulary a
/// tool would use to return it — one content model across the whole
/// loop rather than one per direction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssistantAudioContentChunk {
    /// The discriminator. See [`ContinuationChunk`](super::ContinuationChunk).
    pub r#type: AssistantAudioContentChunkType,
    /// The tool call whose sub-agent produced this chunk; absent on
    /// the main thread. A nested sub-agent names its IMMEDIATE
    /// spawning call, so depth is a chain of ids a caller can follow.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_tool_call_id: Option<String>,
    /// The content itself.
    #[serde(flatten)]
    pub inner: AudioContent,
}

/// [`AssistantAudioContentChunk`]'s discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AssistantAudioContentChunkType {
    #[serde(rename = "assistant_audio_content")]
    #[default]
    AssistantAudioContent,
}
