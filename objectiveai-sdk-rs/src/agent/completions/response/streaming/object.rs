//! Object type for streaming agent completion responses.

use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// The object type for streaming agent completion chunks.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default, JsonSchema, arbitrary::Arbitrary)]
#[schemars(rename = "agent.completions.response.streaming.Object")]
pub enum Object {
    /// A agent completion chunk object.
    #[serde(rename = "agent.completion.chunk")]
    #[default]
    AgentCompletionChunk,
}
