//! The API's assistant message, whole.

use serde::{Deserialize, Serialize};

use super::{ContentBlock, Usage};

/// `BetaMessage`: what one API call produced.
///
/// # One block per stdout record
///
/// Claude Code splits every assistant message into one stdout record
/// per content block before writing it — `content` here is a
/// one-element array on the wire, and reassembling the original
/// message means grouping records by [`id`](Self::id), not by record
/// uuid (those are derived per block).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    /// The message's id — the grouping key for its split-out blocks.
    pub id: String,
    /// Always `message`.
    pub r#type: MessageType,
    /// Always `assistant`.
    pub role: AssistantRole,
    /// The model that produced it. An open set — local slash-command
    /// output rides synthetic assistant messages with a sentinel
    /// model — so a string, as the SDK's own `Model` union
    /// ultimately is.
    pub model: String,
    /// The content, one block per stdout record.
    pub content: Vec<ContentBlock>,
    /// Why generation stopped; `null` mid-stream.
    pub stop_reason: Option<StopReason>,
    /// Which custom stop sequence fired, if one did.
    pub stop_sequence: Option<String>,
    /// What the call billed.
    pub usage: Usage,
    /// Context-management state, injected by Claude Code's own
    /// normalization as `null` when absent. Its shape belongs to an
    /// API newer than the pinned SDK, so it stays unread.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_management: Option<serde_json::Value>,
}

/// [`Message`]'s object-type literal. One variant, because the API
/// says `"message"` and anything else is not this type.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    /// The only value.
    #[default]
    Message,
}

/// [`Message`]'s role literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum AssistantRole {
    /// The only value.
    #[default]
    Assistant,
}

/// Why generation stopped, when it has.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    /// A natural stopping point.
    EndTurn,
    /// The token cap, requested or the model's own.
    MaxTokens,
    /// A custom stop sequence fired.
    StopSequence,
    /// The model invoked one or more tools.
    ToolUse,
}
