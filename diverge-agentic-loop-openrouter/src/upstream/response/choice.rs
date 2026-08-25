//! Choice type for streaming responses.

use serde::{Deserialize, Serialize};

/// A choice in a streaming agent completion chunk.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Choice {
    /// The content delta for this choice.
    pub delta: super::Delta,
    /// The reason generation stopped, if complete.
    pub finish_reason: Option<super::FinishReason>,
    /// The index of this choice.
    pub index: u64,
    /// Log probabilities for tokens, if requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<super::Logprobs>,
}
