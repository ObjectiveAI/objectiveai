//! The `tool.completed` event.

use serde::Deserialize;

/// A tool call finished — name, timing and an error flag, and
/// deliberately nothing more: the gateway never puts the tool's
/// RESULT on this stream (the executor sends it; the emitter drops
/// it). Failures ride this event with [`error`](Self::error) set —
/// there is no `tool.failed`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ToolCompleted {
    /// The discriminator. Always `tool.completed`.
    pub event: ToolCompletedEvent,
    /// The run.
    pub run_id: String,
    /// Seconds since the epoch, fractional.
    pub timestamp: f64,
    /// The tool's name — nullable on the wire.
    #[serde(default)]
    pub tool: Option<String>,
    /// Seconds, rounded to 3 decimals — and a bare integer `0`
    /// when the producer had none, which an `f64` reads too.
    pub duration: f64,
    /// Whether the tool answered as an error.
    pub error: bool,
}

/// [`ToolCompleted`]'s discriminator: the one value no other event
/// carries, which is what lets the union stay untagged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum ToolCompletedEvent {
    /// The only value.
    #[serde(rename = "tool.completed")]
    ToolCompleted,
}
