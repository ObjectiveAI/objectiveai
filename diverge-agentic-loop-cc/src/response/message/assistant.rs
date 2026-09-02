//! The API's assistant message, whole.

use diverge_provider_sdk::endpoints::agentic_loop::run::server::response;
use serde::Deserialize;

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
#[derive(Debug, Clone, PartialEq, Deserialize)]
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
    /// The container the message ran against, when code execution is
    /// in play — written as `null` by every synthetic constructor and
    /// passed through verbatim from the API otherwise.
    pub container: Option<Container>,
    /// Context-management state, injected by Claude Code's own
    /// normalization as `null` when absent.
    pub context_management: Option<ContextManagementResponse>,
}

/// The code-execution container a message ran against.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Container {
    /// The container's id.
    pub id: String,
    /// When it expires, ISO 8601.
    pub expires_at: String,
    /// The skills loaded into it, when any.
    pub skills: Option<Vec<ContainerSkill>>,
}

/// One skill loaded into a code-execution container.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct ContainerSkill {
    /// The skill, by id.
    pub skill_id: String,
    /// Whose skill it is — `anthropic` or `custom` today, and an
    /// API-owned vocabulary, so open.
    pub r#type: String,
    /// The skill's version.
    pub version: String,
}

/// What context management did to the conversation.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ContextManagementResponse {
    /// The edits applied, in order.
    pub applied_edits: Vec<ContextManagementEdit>,
}

/// One context-management edit, discriminated by its dated `type`
/// literal — a vocabulary that grows by design, so an edit newer
/// than this crate lands in [`Other`](Self::Other) verbatim.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum ContextManagementEdit {
    /// Tool uses cleared.
    ClearToolUses {
        /// Always `clear_tool_uses_20250919`.
        r#type: ClearToolUsesEditType,
        /// Input tokens freed.
        cleared_input_tokens: u64,
        /// Tool uses removed.
        cleared_tool_uses: u64,
    },
    /// Thinking cleared.
    ClearThinking {
        /// Always `clear_thinking_20251015`.
        r#type: ClearThinkingEditType,
        /// Input tokens freed.
        cleared_input_tokens: u64,
        /// Thinking turns removed.
        cleared_thinking_turns: u64,
    },
    /// An edit newer than this crate, preserved verbatim.
    Other(serde_json::Value),
}

/// The `clear_tool_uses_20250919` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
pub enum ClearToolUsesEditType {
    /// The only value.
    #[default]
    #[serde(rename = "clear_tool_uses_20250919")]
    ClearToolUses20250919,
}

/// The `clear_thinking_20251015` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
pub enum ClearThinkingEditType {
    /// The only value.
    #[default]
    #[serde(rename = "clear_thinking_20251015")]
    ClearThinking20251015,
}

/// [`Message`]'s object-type literal. One variant, because the API
/// says `"message"` and anything else is not this type.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    /// The only value.
    #[default]
    Message,
}

/// [`Message`]'s role literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
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
/// Serde reads it through [`String`] (`from`), which is what lets
/// the enum stay flat: unit variants and a catch-all cannot share an
/// untagged enum, but a string conversion can name both.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
#[serde(from = "String")]
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

impl Message {
    /// This message's chunks: its blocks', in order.
    ///
    /// The [`usage`](Self::usage) here is deliberately NOT read:
    /// usage reaches the caller exactly once, from the `result`
    /// record — the api crate's doctrine, kept — so a per-message
    /// bill never double-counts. The stop reason, model and
    /// container say nothing the chunk vocabulary can carry.
    pub fn into_chunks(
        self,
        chunks: &mut Vec<response::AgenticLoopChunk>,
        parent_tool_call_id: Option<&str>,
    ) {
        for block in self.content {
            block.into_chunks(chunks, parent_tool_call_id);
        }
    }
}

