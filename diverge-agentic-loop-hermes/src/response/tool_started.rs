//! The `tool.started` event.

use serde::Deserialize;

/// A tool call began.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ToolStarted {
    /// The discriminator. Always `tool.started`.
    pub event: ToolStartedEvent,
    /// The run.
    pub run_id: String,
    /// Seconds since the epoch, fractional.
    pub timestamp: f64,
    /// The tool's name — nullable on the wire.
    #[serde(default)]
    pub tool: Option<String>,
    /// A preview of the ARGUMENTS (never the result), unlimited by
    /// default in Hermes's config — nullable on the wire.
    #[serde(default)]
    pub preview: Option<String>,
}

/// [`ToolStarted`]'s discriminator: the one value no other event
/// carries, which is what lets the union stay untagged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum ToolStartedEvent {
    /// The only value.
    #[serde(rename = "tool.started")]
    ToolStarted,
}
