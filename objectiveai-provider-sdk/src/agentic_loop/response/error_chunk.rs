//! The error chunk.

use serde::{Deserialize, Serialize};

/// A failure.
///
/// In-band rather than a transport error, because a loop can fail
/// AFTER producing output. Ending the stream without saying why would
/// leave the caller holding a partial result and no way to tell it
/// apart from a complete one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ErrorChunk {
    /// The discriminator. See [`ContinuationChunk`](super::ContinuationChunk).
    pub r#type: ErrorChunkType,
    /// HTTP status code.
    pub code: u16,
    /// The message or details, as an arbitrary JSON value — providers
    /// report failures in shapes we do not get to dictate, and
    /// flattening one into a string would discard the structure a
    /// caller needs to act on it.
    pub message: serde_json::Value,
}

/// [`ErrorChunk`]'s discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ErrorChunkType {
    #[serde(rename = "error")]
    #[default]
    Error,
}
