//! Error detail carried on a chunk.

use serde::{Deserialize, Serialize};

/// A failure, reported in-band on the chunk that failed.
///
/// In-band rather than as a transport error because a loop can fail
/// after it has already produced output — the caller needs both the
/// partial result and the reason it stopped.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseError {
    /// HTTP status code.
    pub code: u16,
    /// The message or details, as an arbitrary JSON value.
    pub message: serde_json::Value,
}
