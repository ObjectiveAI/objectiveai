//! A tool result, returned to the loop.

use serde::{Deserialize, Serialize};

use super::{RichContent, ToolRole};

/// The result of one tool call, placed back into the conversation.
///
/// Not streamed in deltas the way an assistant turn is: a tool either
/// returned or it did not, so this arrives whole.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolResponse {
    /// Always `tool`.
    pub role: ToolRole,
    /// Position of this message within the loop, sharing one sequence
    /// with the assistant turns so the conversation has a single
    /// ordering.
    pub index: u64,
    /// The result itself.
    #[serde(flatten)]
    pub inner: ToolMessage,
}

/// What a tool returned, and which call it answers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolMessage {
    /// The result. Same content type as an assistant turn, so a tool
    /// can return images and files rather than only text.
    pub content: RichContent,
    /// The `AssistantToolCallDelta::id` this answers. The only link
    /// back to the call — results may arrive out of order.
    pub tool_call_id: String,
    /// Vendor metadata, passed through untouched.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}
