//! Neutral row types for the `tasks` DB layer — free of the SDK
//! `tasks` command types; the daemon handlers map these to the wire
//! response types.

use objectiveai_sdk::cli::command::AgentArguments;

/// One stored task, as `tasks list` reads it.
#[derive(Debug, Clone)]
pub struct TaskRow {
    pub id: String,
    /// The stored command-request JSON, verbatim.
    pub command: serde_json::Value,
    pub agent_arguments: AgentArguments,
    pub delay_secs: i64,
    pub repeat: bool,
    pub repeat_count: Option<i64>,
    pub run_count: i64,
    pub error_count: i64,
    /// `"success"` / `"error"` / `None` (never ran).
    pub last_result: Option<String>,
    /// `true` = will never fire again (stays listed until deleted).
    pub complete: bool,
    pub created_at: i64,
    pub next_run_at: Option<i64>,
}

/// One task the scheduler just CLAIMED (`scheduled` → `running`) —
/// everything a fire needs.
#[derive(Debug, Clone)]
pub struct ClaimedTask {
    pub id: String,
    pub command: serde_json::Value,
    pub agent_arguments: AgentArguments,
}
