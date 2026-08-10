//! A message from the model.

use rmcp::model::{ContentBlock, MetaObject};
use serde::{Deserialize, Serialize};

use super::ToolCall;

/// A turn the model already took.
///
/// Sent back so the model can see what it said. Content and tool calls
/// are separate fields rather than one interleaved list because they
/// are answered differently: content is read, tool calls are executed
/// and each needs a [`ToolMessage`](super::ToolMessage) in reply.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssistantMessage {
    /// The discriminator. See [`Message`](super::Message).
    pub role: AssistantRole,
    /// What the model said. Empty when the turn was tool calls alone.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content: Vec<ContentBlock>,
    /// What the model called. Empty when the turn was content alone.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCall>,
    /// Arbitrary protocol-level metadata, MCP's `_meta` extension bag.
    #[serde(rename = "_meta", default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<MetaObject>,
}

/// [`AssistantMessage`]'s discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AssistantRole {
    #[serde(rename = "assistant")]
    #[default]
    Assistant,
}
