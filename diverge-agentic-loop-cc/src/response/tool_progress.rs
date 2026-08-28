//! The `tool_progress` records: a long tool run, still running.

use serde::{Deserialize, Serialize};

/// A `type: "tool_progress"` record: a heartbeat for a tool call
/// that is taking a while. Emitted only on remote/container builds
/// (the source gates it on its remote environment markers), and
/// throttled to one per interval per spawning call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolProgress {
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    /// The record's own id.
    pub uuid: String,
    /// The session.
    pub session_id: String,
}
