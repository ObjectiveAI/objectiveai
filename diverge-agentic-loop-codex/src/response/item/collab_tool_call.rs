//! The `collab_tool_call` item: a call to a collab tool.

use std::collections::BTreeMap;

use serde::Deserialize;

/// A call to a collab tool — Codex's own sub-agent machinery, one
/// thread spawning, prompting, waiting on or closing another.
/// Started when invoked, completed when the tool reports. What the
/// harness makes of it is the converter's; the type carries all of
/// it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CollabToolCall {
    /// Which collab tool.
    pub tool: CollabTool,
    /// The calling thread.
    pub sender_thread_id: String,
    /// The threads addressed.
    pub receiver_thread_ids: Vec<String>,
    /// The prompt sent, when the tool sends one.
    pub prompt: Option<String>,
    /// The last known state of each agent involved, by thread id.
    pub agents_states: BTreeMap<String, CollabAgentState>,
    /// Where the call stands.
    pub status: CollabToolCallStatus,
}

/// The collab tools.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollabTool {
    /// Start another agent.
    SpawnAgent,
    /// Send it input.
    SendInput,
    /// Wait for it.
    Wait,
    /// Close it.
    CloseAgent,
}

/// The status of a collab tool call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CollabToolCallStatus {
    /// Running.
    #[default]
    InProgress,
    /// Done.
    Completed,
    /// Failed.
    Failed,
}

/// The state of a collab agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollabAgentStatus {
    /// Spawned, not yet initialized.
    PendingInit,
    /// Working.
    Running,
    /// Interrupted.
    Interrupted,
    /// Finished.
    Completed,
    /// Failed.
    Errored,
    /// Shut down.
    Shutdown,
    /// No such agent.
    NotFound,
}

/// The last known state of a collab agent.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CollabAgentState {
    /// Its status.
    pub status: CollabAgentStatus,
    /// Its last message, when it had one.
    pub message: Option<String>,
}
