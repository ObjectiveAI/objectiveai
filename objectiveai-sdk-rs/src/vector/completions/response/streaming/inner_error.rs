use std::borrow::Cow;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error;

/// An error from a single agent completion inside a [`VectorCompletionChunk`](super::VectorCompletionChunk).
///
/// Yielded by [`VectorCompletionChunk::inner_errors`](super::VectorCompletionChunk::inner_errors).
/// Identifies the failing completion by its `index` (matching
/// [`AgentCompletionChunk::index`](super::AgentCompletionChunk::index)) and
/// carries the underlying [`ResponseError`](error::ResponseError) as a `Cow`
/// so iteration is zero-allocation (`Cow::Borrowed`) while deserialization
/// always lands in `Cow::Owned`.
///
/// Wire shape:
/// ```json
/// { "agent_completion_index": 1, "error": { ... } }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "vector.completions.response.streaming.InnerError")]
pub struct InnerError<'a> {
    /// Index of the failing completion (matches `AgentCompletionChunk::index`).
    pub agent_completion_index: u64,
    /// The underlying error from the agent completion.
    pub error: Cow<'a, error::ResponseError>,
}
