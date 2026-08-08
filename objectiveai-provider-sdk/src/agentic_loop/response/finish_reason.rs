//! Why a turn stopped generating.

use serde::{Deserialize, Serialize};

/// The reason the model stopped generating.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum FinishReason {
    /// A natural stop point or stop sequence.
    #[serde(rename = "stop")]
    Stop,
    /// The maximum token limit.
    #[serde(rename = "length")]
    Length,
    /// The model chose to call one or more tools.
    #[serde(rename = "tool_calls")]
    ToolCalls,
    /// Filtered by content policy.
    #[serde(rename = "content_filter")]
    ContentFilter,
    /// Generation failed.
    #[serde(rename = "error")]
    #[default]
    Error,
}
