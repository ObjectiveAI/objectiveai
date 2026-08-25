//! Tool messages.

use super::super::RichContent;
use serde::{Deserialize, Serialize};

/// A tool message containing the result of a tool call.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
)]
pub struct ToolMessage {
    /// The content of the tool response.
    pub content: RichContent,
    /// The ID of the tool call this message responds to.
    pub tool_call_id: String,
}
