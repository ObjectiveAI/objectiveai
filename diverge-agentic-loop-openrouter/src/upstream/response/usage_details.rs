//! Token detail breakdowns.

use serde::{Deserialize, Serialize};

/// Detailed breakdown of completion token usage.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    Default,
)]
pub struct CompletionTokensDetails {
    /// Tokens from accepted predictions (speculative decoding).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accepted_prediction_tokens: Option<u64>,
    /// Audio output tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_tokens: Option<u64>,
    /// Tokens used for reasoning/thinking.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_tokens: Option<u64>,
    /// Tokens from rejected predictions (speculative decoding).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rejected_prediction_tokens: Option<u64>,
}

/// Detailed breakdown of prompt token usage.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    Default,
)]
pub struct PromptTokensDetails {
    /// Audio input tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_tokens: Option<u64>,
    /// Tokens served from cache.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_tokens: Option<u64>,
    /// Tokens written to cache.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_write_tokens: Option<u64>,
    /// Video input tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_tokens: Option<u64>,
}
