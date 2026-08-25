//! Streaming chat completion chunks from OpenRouter.

use serde::{Deserialize, Serialize};

/// A streaming chat completion chunk from OpenRouter.
///
/// Contains partial response data that arrives incrementally during streaming.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChatCompletionChunk {
    /// Unique identifier for this completion from OpenRouter.
    pub id: String,
    /// Completion choices containing the generated content.
    pub choices: Vec<super::Choice>,
    /// Unix timestamp when the completion was created.
    pub created: u64,
    /// The model that generated this completion.
    pub model: String,
    /// Object type indicator.
    pub object: super::Object,
    /// The service tier used for this request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<String>,
    /// System fingerprint for reproducibility.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<String>,
    /// Token usage statistics (typically in the final chunk).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<super::Usage>,
    /// The upstream provider that served this request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
}
