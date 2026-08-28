//! The `tool_use_summary` records: tool calls, summarized after.

use serde::Deserialize;

/// A `type: "tool_use_summary"` record: a one-line summary of a run
/// of preceding tool calls, named by id.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ToolUseSummary {
    /// Always `tool_use_summary`.
    pub r#type: ToolUseSummaryType,
    /// The summary text.
    pub summary: String,
    /// The calls it summarizes.
    pub preceding_tool_use_ids: Vec<String>,
    /// The record's own id.
    pub uuid: String,
    /// The session.
    pub session_id: String,
}

/// The `tool_use_summary` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ToolUseSummaryType {
    /// The only value.
    #[default]
    ToolUseSummary,
}
