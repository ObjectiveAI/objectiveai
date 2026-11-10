//! The `subagent.complete` event.

use serde::Deserialize;

/// A delegated child agent finished — the same gateway allowlist
/// as [`SubagentStart`](super::SubagentStart), duplicated
/// deliberately because the wire has two events, and this is the
/// one whose accounting fields (tokens, cost, duration, files) are
/// actually filled. Every field beyond the trio is ABSENT when the
/// producer had nothing — never null.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct SubagentComplete {
    /// The discriminator. Always `subagent.complete`.
    pub event: SubagentCompleteEvent,
    /// The run.
    pub run_id: String,
    /// Seconds since the epoch, fractional.
    pub timestamp: f64,
    /// A short display line, redacted upstream.
    #[serde(default)]
    pub preview: Option<String>,
    /// The child's goal, redacted upstream.
    #[serde(default)]
    pub goal: Option<String>,
    /// How many sibling tasks the delegation carried.
    #[serde(default)]
    pub task_count: Option<u64>,
    /// This task's position among them.
    #[serde(default)]
    pub task_index: Option<u64>,
    /// The child's id.
    #[serde(default)]
    pub subagent_id: Option<String>,
    /// The child's session.
    #[serde(default)]
    pub child_session_id: Option<String>,
    /// The parent's id.
    #[serde(default)]
    pub parent_id: Option<String>,
    /// Spawn depth.
    #[serde(default)]
    pub depth: Option<u64>,
    /// The child's model.
    #[serde(default)]
    pub model: Option<String>,
    /// How many tools the child carried.
    #[serde(default)]
    pub tool_count: Option<u64>,
    /// How it ended — `timeout`, `error`, `failed`, or the entry's
    /// own status word.
    #[serde(default)]
    pub status: Option<String>,
    /// A summary, redacted and capped at 500 chars upstream.
    #[serde(default)]
    pub summary: Option<String>,
    /// Seconds, two decimals.
    #[serde(default)]
    pub duration_seconds: Option<f64>,
    /// Input tokens, int-coerced upstream.
    #[serde(default)]
    pub input_tokens: Option<u64>,
    /// Output tokens, int-coerced upstream.
    #[serde(default)]
    pub output_tokens: Option<u64>,
    /// Reasoning tokens, int-coerced upstream.
    #[serde(default)]
    pub reasoning_tokens: Option<u64>,
    /// API calls, int-coerced upstream.
    #[serde(default)]
    pub api_calls: Option<u64>,
    /// Cost in dollars — omitted upstream when unknowable.
    #[serde(default)]
    pub cost_usd: Option<f64>,
    /// Files the child read, capped at 40 upstream.
    #[serde(default)]
    pub files_read: Option<Vec<String>>,
    /// Files the child wrote, capped at 40 upstream.
    #[serde(default)]
    pub files_written: Option<Vec<String>>,
    /// The tail of the child's tool activity — today objects of
    /// `{tool, preview, is_error}`, but forwarded raw with no
    /// schema enforced at the gateway, so tolerated as values.
    #[serde(default)]
    pub output_tail: Option<Vec<serde_json::Value>>,
}

/// [`SubagentComplete`]'s discriminator: the one value no other
/// event carries, which is what lets the union stay untagged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum SubagentCompleteEvent {
    /// The only value.
    #[serde(rename = "subagent.complete")]
    SubagentComplete,
}
