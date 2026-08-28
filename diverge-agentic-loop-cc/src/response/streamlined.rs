//! The streamlined records: the internal terse output mode.

use serde::{Deserialize, Serialize};

/// A `type: "streamlined_text"` record: an assistant message reduced
/// to its text. Internal, double-gated behind a build flag and an
/// environment opt-in; in that mode these REPLACE assistant records.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StreamlinedText {
    /// The text kept from the assistant message.
    pub text: String,
    /// The record's own id.
    pub uuid: String,
    /// The session.
    pub session_id: String,
}

/// A `type: "streamlined_tool_use_summary"` record: the mode's
/// cumulative stand-in for tool_use blocks.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StreamlinedToolUseSummary {
    /// The cumulative summary of tool calls so far.
    pub tool_summary: String,
    /// The record's own id.
    pub uuid: String,
    /// The session.
    pub session_id: String,
}
