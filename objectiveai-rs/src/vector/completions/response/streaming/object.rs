//! Object type marker for streaming vector completion chunks.

use serde::{Deserialize, Serialize};

/// Object type for streaming vector completion chunks.
///
/// Serializes to `"vector.completion.chunk"` in JSON.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub enum Object {
    /// A streaming vector completion chunk.
    #[serde(rename = "vector.completion.chunk")]
    #[default]
    VectorCompletionChunk,
}
