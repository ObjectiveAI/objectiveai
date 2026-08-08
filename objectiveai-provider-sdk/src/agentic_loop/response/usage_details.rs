//! Token and cost breakdowns, shared by [`Usage`](super::Usage) and
//! [`UpstreamUsage`](super::UpstreamUsage).

use serde::{Deserialize, Serialize};

use super::util;

/// Where the completion tokens went.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CompletionTokensDetails {
    /// Tokens from accepted speculative predictions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accepted_prediction_tokens: Option<u64>,
    /// Audio output tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_tokens: Option<u64>,
    /// Tokens spent reasoning.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_tokens: Option<u64>,
    /// Tokens from rejected speculative predictions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rejected_prediction_tokens: Option<u64>,
}

impl CompletionTokensDetails {
    /// Whether any count is non-zero.
    pub fn any_usage(&self) -> bool {
        self.accepted_prediction_tokens.is_some_and(|v| v > 0)
            || self.audio_tokens.is_some_and(|v| v > 0)
            || self.reasoning_tokens.is_some_and(|v| v > 0)
            || self.rejected_prediction_tokens.is_some_and(|v| v > 0)
    }

    /// Sum another breakdown into this one, field by field.
    pub fn push(&mut self, other: &CompletionTokensDetails) {
        util::push_option_u64(
            &mut self.accepted_prediction_tokens,
            &other.accepted_prediction_tokens,
        );
        util::push_option_u64(&mut self.audio_tokens, &other.audio_tokens);
        util::push_option_u64(
            &mut self.reasoning_tokens,
            &other.reasoning_tokens,
        );
        util::push_option_u64(
            &mut self.rejected_prediction_tokens,
            &other.rejected_prediction_tokens,
        );
    }
}

/// Where the prompt tokens came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
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

impl PromptTokensDetails {
    /// Whether any count is non-zero.
    pub fn any_usage(&self) -> bool {
        self.audio_tokens.is_some_and(|v| v > 0)
            || self.cached_tokens.is_some_and(|v| v > 0)
            || self.cache_write_tokens.is_some_and(|v| v > 0)
            || self.video_tokens.is_some_and(|v| v > 0)
    }

    /// Sum another breakdown into this one, field by field.
    pub fn push(&mut self, other: &PromptTokensDetails) {
        util::push_option_u64(&mut self.audio_tokens, &other.audio_tokens);
        util::push_option_u64(&mut self.cached_tokens, &other.cached_tokens);
        util::push_option_u64(
            &mut self.cache_write_tokens,
            &other.cache_write_tokens,
        );
        util::push_option_u64(&mut self.video_tokens, &other.video_tokens);
    }
}

/// Who charged what.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CostDetails {
    /// Charged by the immediate upstream.
    pub upstream_inference_cost: rust_decimal::Decimal,
    /// Charged by the upstream's own upstream — the model provider
    /// behind an aggregator.
    pub upstream_upstream_inference_cost: rust_decimal::Decimal,
}

impl CostDetails {
    /// Whether any cost is non-zero.
    pub fn any_usage(&self) -> bool {
        self.upstream_inference_cost > rust_decimal::Decimal::ZERO
            || self.upstream_upstream_inference_cost
                > rust_decimal::Decimal::ZERO
    }

    /// Sum another breakdown into this one.
    pub fn push(&mut self, other: &CostDetails) {
        self.upstream_inference_cost += other.upstream_inference_cost;
        self.upstream_upstream_inference_cost +=
            other.upstream_upstream_inference_cost;
    }
}
