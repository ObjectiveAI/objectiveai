//! Deterministic-script override types for the mock agent.
//!
//! When [`super::AgentBase::calls`] is `Some(vec![Call{...}, ...])`,
//! the mock agent skips its usual per-turn RNG response selection and
//! instead emits each [`Call`] as its own assistant turn — every
//! [`CallToolCall`] in `tool_calls` first (as tool-call deltas), then
//! `content` (as content deltas) — in array order. Each subsequent
//! turn walks the continuation to count how many [`Call`]s have
//! already been satisfied; the next un-matched [`Call`] is what that
//! turn emits. Once every [`Call`] has been satisfied, the mock falls
//! through to its normal dispatcher.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// One scripted assistant turn for the mock agent's deterministic
/// `calls` override (see [`super::AgentBase::calls`]).
///
/// When the mock fires this `Call`, it emits every entry in
/// `tool_calls` first (as tool-call deltas), then `content` (as
/// content deltas), finishing the turn after the last content chunk.
/// The match check on the continuation compares
/// `(tool_calls[i].name, tool_calls[i].arguments)` pairs plus the
/// full `content` string — call ids are intentionally excluded since
/// they're random per-emit.
#[derive(
    Clone,
    Debug,
    Default,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "agent.mock.Call")]
pub struct Call {
    /// Tool calls emitted by this turn. Empty `Vec` is allowed and
    /// means "no tool calls, just content."
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(extend("omitempty" = true))]
    pub tool_calls: Vec<CallToolCall>,

    /// Assistant text content emitted *after* the tool calls. Plain
    /// `String` — the mock's chunk-emission path only produces
    /// `RichContent::Text`, so multimodal content was never possible
    /// here anyway. An empty string is treated as "no content" by
    /// the emitter (the wire shape becomes `content: None`) and
    /// matches an assistant message with `content: None`.
    pub content: String,
}

/// A single tool call within a [`Call`]. Identifies the tool by
/// `name` and carries the exact JSON-encoded arguments to pass. No
/// `id` field — call ids on the wire are minted randomly per emit
/// and are ignored by the override's match check.
#[derive(
    Clone,
    Debug,
    Default,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "agent.mock.CallToolCall")]
pub struct CallToolCall {
    /// Tool name. Matched against the upstream tool surface at call
    /// time the same way a model-emitted tool call would be.
    pub name: String,
    /// JSON-string arguments — same wire shape as
    /// `AssistantToolCallFunction::arguments`. Passed through to the
    /// emitted tool call verbatim; validation is the downstream
    /// `tools/call` handler's job.
    pub arguments: String,
}
