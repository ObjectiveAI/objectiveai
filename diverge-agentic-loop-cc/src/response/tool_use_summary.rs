//! The `tool_use_summary` records: tool calls, summarized after.

use serde::{Deserialize, Serialize};

/// A `type: "tool_use_summary"` record: a one-line summary of a run
/// of preceding tool calls, named by id.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolUseSummary {
    /// The summary text.
    pub summary: String,
    /// The calls it summarizes.
    pub preceding_tool_use_ids: Vec<String>,
    /// The record's own id.
    pub uuid: String,
    /// The session.
    pub session_id: String,
}
