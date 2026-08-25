//! Delta type for streaming responses.

use serde::{Deserialize, Serialize};

/// A delta (incremental update) in a streaming response.
///
/// Each field contains only the new content since the last delta.
/// Deltas can be accumulated using the [`push`](Self::push) method.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Delta {
    /// New content text since the last delta.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// New refusal text since the last delta.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refusal: Option<String>,
    /// The role (only present in the first delta).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<super::Role>,
    /// Tool call updates.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls:
        Option<Vec<crate::upstream::request::AssistantToolCallDelta>>,

    /// New reasoning text since the last delta.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,
    /// New generated images.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<super::Image>>,
}
