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
///
/// # Eight knowns and an open tail, because the vocabulary is the API's
///
/// Claude Code copies this field verbatim from the API response and
/// never validates it, so its value set belongs to Anthropic's
/// servers — a moving target no pinned source can close. The pinned
/// SDK's d.ts knew four values; Claude Code's own code branches on
/// two more (`refusal`, `model_context_window_exceeded`); the current
/// API documents eight. Those eight are named, and anything newer
/// lands in [`Other`](Self::Other) with the string preserved
/// verbatim — a future value degrades to an unrecognized reason
/// instead of an unparseable line.
///
/// Serde goes through [`String`] both ways (`from`/`into`), which is
/// what lets the enum stay flat: unit variants and a catch-all
/// cannot share an untagged enum, but a string conversion can name
/// both.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub enum StopReason {
    /// A natural stopping point.
    EndTurn,
    /// The token cap, requested or the model's own.
    MaxTokens,
    /// A custom stop sequence fired.
    StopSequence,
    /// The model invoked one or more tools.
    ToolUse,
    /// A long turn paused, resumable.
    PauseTurn,
    /// The API compacted the context.
    Compaction,
    /// The model refused.
    Refusal,
    /// The model's context window was exceeded.
    ModelContextWindowExceeded,
    /// A reason newer than this crate, preserved verbatim.
    Other(String),
}

impl From<String> for StopReason {
    fn from(value: String) -> Self {
        match value.as_str() {
            "end_turn" => StopReason::EndTurn,
            "max_tokens" => StopReason::MaxTokens,
            "stop_sequence" => StopReason::StopSequence,
            "tool_use" => StopReason::ToolUse,
            "pause_turn" => StopReason::PauseTurn,
            "compaction" => StopReason::Compaction,
            "refusal" => StopReason::Refusal,
            "model_context_window_exceeded" => {
                StopReason::ModelContextWindowExceeded
            }
            _ => StopReason::Other(value),
        }
    }
}

impl From<StopReason> for String {
    fn from(value: StopReason) -> Self {
        match value {
            StopReason::EndTurn => "end_turn".to_string(),
            StopReason::MaxTokens => "max_tokens".to_string(),
            StopReason::StopSequence => "stop_sequence".to_string(),
            StopReason::ToolUse => "tool_use".to_string(),
            StopReason::PauseTurn => "pause_turn".to_string(),
            StopReason::Compaction => "compaction".to_string(),
            StopReason::Refusal => "refusal".to_string(),
            StopReason::ModelContextWindowExceeded => {
                "model_context_window_exceeded".to_string()
            }
            StopReason::Other(value) => value,
        }
    }
}
