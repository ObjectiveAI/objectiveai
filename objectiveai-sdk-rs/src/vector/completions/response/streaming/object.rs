//! Object type marker for streaming vector completion chunks.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Object type for streaming vector completion chunks.
///
/// Serializes to `"vector.completion.chunk"` in JSON.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Serialize,
    Deserialize,
    Default,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "vector.completions.response.streaming.Object")]
pub enum Object {
    /// A streaming vector completion chunk.
    #[serde(rename = "vector.completion.chunk")]
    #[default]
    VectorCompletionChunk,
}
