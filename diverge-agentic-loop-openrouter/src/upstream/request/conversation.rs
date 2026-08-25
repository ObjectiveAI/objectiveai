//! A message in the conversation.

use super::{AssistantMessage, ToolMessage, UserMessage};
use serde::{Deserialize, Serialize};

/// A message in the conversation.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
)]
#[serde(tag = "role")]
pub enum Message {
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
