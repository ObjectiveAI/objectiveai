//! Token usage and cost information.

use crate::chat::completions::response::util;
use serde::{Deserialize, Serialize};

/// Token usage and cost statistics for a completion.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Usage {
    /// Number of tokens in the completion.
    pub completion_tokens: u64,
    /// Number of tokens in the prompt.
    pub prompt_tokens: u64,
    /// Total tokens (prompt + completion).
    pub total_tokens: u64,
    /// Detailed breakdown of completion tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_tokens_details: Option<CompletionTokensDetails>,
    /// Detailed breakdown of prompt tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens_details: Option<PromptTokensDetails>,
    /// The cost charged by ObjectiveAI for this request.
    pub cost: rust_decimal::Decimal,
    /// Detailed cost breakdown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_details: Option<CostDetails>,
    /// Total cost including ObjectiveAI's charge plus all upstream charges.
    /// For BYOK requests, ObjectiveAI only charges the cost_multiplier difference,
    /// but total_cost still includes what the upstream provider charged.
    pub total_cost: rust_decimal::Decimal,
    /// The multiplier applied to compute ObjectiveAI's charge.
    pub cost_multiplier: rust_decimal::Decimal,
    /// Whether this request used Bring Your Own Key (BYOK).
    pub is_byok: bool,
}

impl Usage {
    /// Appends usage statistics from another instance.
    pub fn push(&mut self, other: &Usage) {
        self.completion_tokens += other.completion_tokens;
        self.prompt_tokens += other.prompt_tokens;
        self.total_tokens += other.total_tokens;
        match (
            &mut self.completion_tokens_details,
            &other.completion_tokens_details,
        ) {
            (Some(self_details), Some(other_details)) => {
                self_details.push(other_details);
            }
            (None, Some(other_details)) => {
                self.completion_tokens_details = Some(other_details.clone());
            }
            _ => {}
        }
        match (
            &mut self.prompt_tokens_details,
            &other.prompt_tokens_details,
        ) {
            (Some(self_details), Some(other_details)) => {
                self_details.push(other_details);
            }
            (None, Some(other_details)) => {
                self.prompt_tokens_details = Some(other_details.clone());
            }
            _ => {}
        }
        self.cost += other.cost;
        match (&mut self.cost_details, &other.cost_details) {
            (Some(self_cost_details), Some(other_cost_details)) => {
                self_cost_details.push(other_cost_details);
            }
            (None, Some(other_cost_details)) => {
                self.cost_details = Some(other_cost_details.clone());
            }
            _ => {}
        }
        self.total_cost += other.total_cost;
    }
}

/// Detailed breakdown of completion token usage.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostDetails {
    /// Cost charged by the immediate upstream (e.g., OpenRouter).
    pub upstream_inference_cost: rust_decimal::Decimal,
    /// Cost charged by the upstream's upstream (e.g., the actual model provider).
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
