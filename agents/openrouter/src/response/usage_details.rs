//! Token detail breakdowns.

use serde::Deserialize;

/// Detailed breakdown of completion token usage.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Deserialize,
    Default,
)]
pub struct CompletionTokensDetails {
    /// Tokens from accepted predictions (speculative decoding).
    pub accepted_prediction_tokens: Option<u64>,
    /// Audio output tokens.
    pub audio_tokens: Option<u64>,
    /// Tokens used for reasoning/thinking.
    pub reasoning_tokens: Option<u64>,
    /// Tokens from rejected predictions (speculative decoding).
    pub rejected_prediction_tokens: Option<u64>,
}

/// Detailed breakdown of prompt token usage.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Deserialize,
    Default,
)]
pub struct PromptTokensDetails {
    /// Audio input tokens.
    pub audio_tokens: Option<u64>,
    /// Tokens served from cache.
    pub cached_tokens: Option<u64>,
    /// Tokens written to cache.
    pub cache_write_tokens: Option<u64>,
    /// Video input tokens.
    pub video_tokens: Option<u64>,
}
