//! Finish reason for agent completions.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The reason the model stopped generating.
#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    Default,
    PartialEq,
    Eq,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "agent.completions.response.FinishReason")]
pub enum FinishReason {
    /// The model reached a natural stop point or stop sequence.
    #[schemars(title = "Stop")]
    #[serde(rename = "stop")]
    Stop,
    /// The model reached the maximum token limit.
    #[schemars(title = "Length")]
    #[serde(rename = "length")]
    Length,
    /// The model decided to call one or more tools.
    #[schemars(title = "ToolCalls")]
    #[serde(rename = "tool_calls")]
    ToolCalls,
    /// The response was filtered due to content policy.
    #[schemars(title = "ContentFilter")]
    #[serde(rename = "content_filter")]
    ContentFilter,
    /// An error occurred during generation.
    #[schemars(title = "Error")]
    #[serde(rename = "error")]
    #[default]
    Error,
}
