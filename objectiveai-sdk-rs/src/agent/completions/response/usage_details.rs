//! Shared token and cost detail types used by both [`Usage`](super::Usage) and
//! [`UpstreamUsage`](super::UpstreamUsage).

use super::util;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Detailed breakdown of completion token usage.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    Default,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "agent.completions.response.CompletionTokensDetails")]
pub struct CompletionTokensDetails {
    /// Tokens from accepted predictions (speculative decoding).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_option_u64)]
    pub accepted_prediction_tokens: Option<u64>,
    /// Audio output tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_option_u64)]
    pub audio_tokens: Option<u64>,
    /// Tokens used for reasoning/thinking.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_option_u64)]
    pub reasoning_tokens: Option<u64>,
    /// Tokens from rejected predictions (speculative decoding).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_option_u64)]
    pub rejected_prediction_tokens: Option<u64>,
}

impl CompletionTokensDetails {
    /// Returns `true` if any token count is non-zero.
    pub fn any_usage(&self) -> bool {
        self.accepted_prediction_tokens.is_some_and(|v| v > 0)
            || self.audio_tokens.is_some_and(|v| v > 0)
            || self.reasoning_tokens.is_some_and(|v| v > 0)
            || self.rejected_prediction_tokens.is_some_and(|v| v > 0)
    }

    /// Appends token details from another instance.
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

/// Detailed breakdown of prompt token usage.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    Default,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "agent.completions.response.PromptTokensDetails")]
pub struct PromptTokensDetails {
    /// Audio input tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_option_u64)]
    pub audio_tokens: Option<u64>,
    /// Tokens served from cache.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_option_u64)]
    pub cached_tokens: Option<u64>,
    /// Tokens written to cache.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_option_u64)]
    pub cache_write_tokens: Option<u64>,
    /// Video input tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_option_u64)]
    pub video_tokens: Option<u64>,
}

impl PromptTokensDetails {
    /// Returns `true` if any token count is non-zero.
    pub fn any_usage(&self) -> bool {
        self.audio_tokens.is_some_and(|v| v > 0)
            || self.cached_tokens.is_some_and(|v| v > 0)
            || self.cache_write_tokens.is_some_and(|v| v > 0)
            || self.video_tokens.is_some_and(|v| v > 0)
    }

    /// Appends token details from another instance.
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

/// Detailed cost breakdown.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "agent.completions.response.CostDetails")]
pub struct CostDetails {
    /// Cost charged by the immediate upstream (e.g., OpenRouter).
    #[serde(deserialize_with = "crate::serde_util::decimal")]
    #[schemars(with = "f64")]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_rust_decimal)]
    pub upstream_inference_cost: rust_decimal::Decimal,
    /// Cost charged by the upstream's upstream (e.g., the actual model provider).
    #[serde(deserialize_with = "crate::serde_util::decimal")]
    #[schemars(with = "f64")]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_rust_decimal)]
    pub upstream_upstream_inference_cost: rust_decimal::Decimal,
}

impl CostDetails {
    /// Returns `true` if any cost is non-zero.
    pub fn any_usage(&self) -> bool {
        self.upstream_inference_cost > rust_decimal::Decimal::ZERO
            || self.upstream_upstream_inference_cost
                > rust_decimal::Decimal::ZERO
    }

    /// Appends cost details from another instance.
    pub fn push(&mut self, other: &CostDetails) {
        self.upstream_inference_cost += other.upstream_inference_cost;
        self.upstream_upstream_inference_cost +=
            other.upstream_upstream_inference_cost;
    }
}
