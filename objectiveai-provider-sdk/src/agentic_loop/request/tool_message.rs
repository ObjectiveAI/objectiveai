//! A tool result, fed back to the model.

use rmcp::model::CallToolResult;
use serde::{Deserialize, Serialize};

/// What a tool returned, in reply to a
/// [`ToolCall`](super::ToolCall).
///
/// Carries MCP's [`CallToolResult`] verbatim — including `isError`, so
/// a tool that FAILED is still a message the model gets to read and
/// react to, rather than something the loop has to hide or turn into
/// an error of its own.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolMessage {
    /// The discriminator. See [`Message`](super::Message).
    pub role: ToolRole,
    /// The [`ToolCall::id`](super::ToolCall::id) this answers.
    pub id: String,
    /// The result itself.
    #[serde(flatten)]
    pub inner: CallToolResult,
}

/// [`ToolMessage`]'s discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ToolRole {
    #[serde(rename = "tool")]
    #[default]
    Tool,
}
