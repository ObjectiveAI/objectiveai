//! The inner `event` payload emitted by the gemini runner on each
//! stdout `{"type":"event","id":..,"event":<GeminiEvent>}` line.
//!
//! The runner tags each event with a `kind` discriminator. Unlike the
//! codex runner (which streams coarse `thread.*` events), the gemini
//! runner streams fine-grained content deltas plus the tool loop it
//! runs internally:
//!
//! - `text`        — an assistant content delta.
//! - `thinking`    — an assistant reasoning delta.
//! - `tool_use`    — the model called a tool (the runner dispatches it
//!   internally; this is informational so the downstream consumer sees
//!   the call).
//! - `tool_result` — the result of a `tool_use` the runner dispatched.
//! - `usage`       — token accounting, emitted once near the end.
//!
//! Unknown `kind` values deserialize to [`GeminiEvent::Unknown`] and
//! are dropped downstream (forward-compatible).

use serde::{Deserialize, Serialize};

/// One inner `event` payload from the gemini runner, discriminated by
/// `kind`. Mirrors the dict shapes constructed in the runner's
/// `main.py` `emit_event` calls 1:1.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GeminiEvent {
    /// `{"kind":"text","text":..}` — assistant content delta.
    Text { text: String },
    /// `{"kind":"thinking","text":..}` — assistant reasoning delta.
    Thinking { text: String },
    /// `{"kind":"tool_use","id":..,"name":..,"input":{..}}` — the model
    /// called a tool. The gemini runner dispatches it internally.
    ToolUse {
        id: String,
        name: String,
        #[serde(default)]
        input: serde_json::Value,
    },
    /// `{"kind":"tool_result","tool_use_id":..,"content":str,"is_error":bool}`
    /// — the result of a runner-dispatched `tool_use`.
    ToolResult {
        tool_use_id: String,
        #[serde(default)]
        content: String,
        #[serde(default)]
        is_error: bool,
    },
    /// `{"kind":"usage","input_tokens":..,"output_tokens":..,"thinking_tokens":..,"total_tokens":..}`
    /// — token accounting, emitted once near the end.
    Usage {
        #[serde(default)]
        input_tokens: u64,
        #[serde(default)]
        output_tokens: u64,
        #[serde(default)]
        thinking_tokens: u64,
        #[serde(default)]
        total_tokens: u64,
    },
    /// Forward-compatible catch-all for unrecognized `kind` values.
    #[serde(other)]
    Unknown,
}
