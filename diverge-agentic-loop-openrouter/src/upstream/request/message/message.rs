//! A message in an OpenRouter request body.

use serde::{Deserialize, Serialize};

use super::{
    AssistantMessage, DeveloperMessage, SystemMessage, ToolMessage,
    UserMessage,
};

/// A message in an OpenRouter request body.
///
/// Tagged by `role`, so each variant serializes to
/// `{"role": ..., ...}` with the variant's own fields beside the tag.
/// The agent's system prompt is the leading `system` or `developer`
/// entry; the conversation follows as `user`, `assistant` and `tool`
/// messages.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "role")]
pub enum Message {
    /// The agent's system prompt, as a system message.
    #[serde(rename = "system")]
    System(SystemMessage),
    /// The agent's system prompt, as a developer message.
    #[serde(rename = "developer")]
    Developer(DeveloperMessage),
    /// A user message from the end user.
    #[serde(rename = "user")]
    User(UserMessage),
    /// An assistant message (model's previous response).
    #[serde(rename = "assistant")]
    Assistant(AssistantMessage),
    /// A tool message containing the result of a tool call.
    #[serde(rename = "tool")]
    Tool(ToolMessage),
}
