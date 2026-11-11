//! The streamlined records: the internal terse output mode.

use serde::Deserialize;

/// A `type: "streamlined_text"` record: an assistant message reduced
/// to its text. Internal, double-gated behind a build flag and an
/// environment opt-in; in that mode these REPLACE assistant records.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct StreamlinedText {
    /// Always `streamlined_text`.
    pub r#type: StreamlinedTextType,
    /// The text kept from the assistant message.
    pub text: String,
    /// The record's own id.
    pub uuid: String,
    /// The session.
    pub session_id: String,
}

/// A `type: "streamlined_tool_use_summary"` record: the mode's
/// cumulative stand-in for tool_use blocks.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct StreamlinedToolUseSummary {
    /// Always `streamlined_tool_use_summary`.
    pub r#type: StreamlinedToolUseSummaryType,
    /// The cumulative summary of tool calls so far.
    pub tool_summary: String,
    /// The record's own id.
    pub uuid: String,
    /// The session.
    pub session_id: String,
}

/// The `streamlined_text` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum StreamlinedTextType {
    /// The only value.
    #[default]
    StreamlinedText,
}

/// The `streamlined_tool_use_summary` literal.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum StreamlinedToolUseSummaryType {
    /// The only value.
    #[default]
    StreamlinedToolUseSummary,
}
