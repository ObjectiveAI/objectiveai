//! Stream options for OpenRouter requests.

use serde::{Deserialize, Serialize};

/// Options for streaming responses.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct StreamOptions {
    /// Whether to include usage statistics in the final chunk.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_usage: Option<bool>,
}
