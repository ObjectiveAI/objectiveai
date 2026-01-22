//! Usage statistics for vector completions.

use crate::chat;
use serde::{Deserialize, Serialize};

/// Aggregated token and cost usage for a vector completion.
///
/// Since vector completions run multiple chat completions (one per LLM in the
/// ensemble), this struct aggregates usage across all underlying completions.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Usage {
    /// Total tokens generated across all completions.
    pub completion_tokens: u64,
    /// Total prompt tokens across all completions.
    pub prompt_tokens: u64,
    /// Sum of completion and prompt tokens.
    pub total_tokens: u64,
    /// Breakdown of completion tokens (reasoning, audio, etc.) if available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_tokens_details:
        Option<chat::completions::response::CompletionTokensDetails>,
    /// Breakdown of prompt tokens (cached, audio, etc.) if available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens_details:
        Option<chat::completions::response::PromptTokensDetails>,
    /// Cost charged by ObjectiveAI for this request.
    pub cost: rust_decimal::Decimal,
    /// Breakdown of upstream and upstream_upstream costs if available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_details: Option<chat::completions::response::CostDetails>,
    /// Total cost including upstream provider charges. Only differs from `cost`
    /// when using BYOK (Bring Your Own Key).
    pub total_cost: rust_decimal::Decimal,
}

impl Usage {
    /// Returns `true` if any usage metrics are non-zero.
    pub fn any_usage(&self) -> bool {
        self.completion_tokens > 0
            || self.prompt_tokens > 0
            || self.total_tokens > 0
            || self.completion_tokens_details.as_ref().is_some_and(
                chat::completions::response::CompletionTokensDetails::any_usage,
            )
            || self.prompt_tokens_details.as_ref().is_some_and(
                chat::completions::response::PromptTokensDetails::any_usage,
            )
            || self.cost > rust_decimal::Decimal::ZERO
            || self.cost_details.as_ref().is_some_and(
                chat::completions::response::CostDetails::any_usage,
            )
            || self.total_cost > rust_decimal::Decimal::ZERO
    }

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
            (Some(self_details), Some(other_details)) => {
                self_details.push(other_details);
            }
            (None, Some(other_details)) => {
                self.cost_details = Some(other_details.clone());
            }
            _ => {}
        }
        self.total_cost += other.total_cost;
    }

    /// Appends usage from a chat completion response.
    pub fn push_chat_completion_usage(
        &mut self,
        other: &chat::completions::response::Usage,
    ) {
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
            (Some(self_details), Some(other_details)) => {
                self_details.push(other_details);
            }
            (None, Some(other_details)) => {
                self.cost_details = Some(other_details.clone());
            }
            _ => {}
        }
        self.total_cost += other.total_cost;
    }
}
