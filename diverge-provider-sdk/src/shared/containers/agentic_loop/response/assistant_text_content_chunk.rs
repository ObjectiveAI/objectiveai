//! The assistant text content chunk.

use rmcp::model::TextContent;
use serde::{Deserialize, Serialize};

use super::Logprob;

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
    /// The discriminator. See [`AgenticLoopChunk`](super::AgenticLoopChunk).
    pub r#type: AssistantTextContentChunkType,
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

impl AssistantTextContentChunk {
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
