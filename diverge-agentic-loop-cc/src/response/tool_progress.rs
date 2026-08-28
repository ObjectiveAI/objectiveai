//! The `tool_progress` records: a long tool run, still running.

use serde::Deserialize;

/// A `type: "tool_progress"` record: a heartbeat for a tool call
/// that is taking a while. Emitted only on remote/container builds
/// (the source gates it on its remote environment markers), and
/// throttled to one per interval per spawning call.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ToolProgress {
    /// Always `tool_progress`.
    pub r#type: ToolProgressType,
    /// The call still running.
    pub tool_use_id: String,
    /// The tool being run.
    pub tool_name: String,
    /// The spawning tool call, for a subagent's record; `null` on
    /// the main thread.
    pub parent_tool_use_id: Option<String>,
    /// How long it has been running.
    pub elapsed_time_seconds: f64,
    /// The background task it belongs to, if any.
    pub task_id: Option<String>,
    /// The record's own id.
    pub uuid: String,
    /// The session.
    pub session_id: String,
}

/// The `tool_progress` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ToolProgressType {
    /// The only value.
    #[default]
    ToolProgress,
}
