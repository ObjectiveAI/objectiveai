//! Aggregated token usage and cost information for agent completions.

use super::{CompletionTokensDetails, CostDetails, PromptTokensDetails, UpstreamUsage};
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// Aggregated token and cost usage for an agent completion.
///
/// This is the "primary" usage type that aggregates across all upstream
/// assistant responses within a single agent completion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default, JsonSchema, arbitrary::Arbitrary)]
#[schemars(rename = "agent.completions.response.Usage")]
pub struct Usage {
    /// Total tokens generated across all assistant responses.
    #[arbitrary(with = crate::arbitrary_util::arbitrary_u64)]
    pub completion_tokens: u64,
    /// Total prompt tokens across all assistant responses.
    #[arbitrary(with = crate::arbitrary_util::arbitrary_u64)]
    pub prompt_tokens: u64,
    /// Sum of completion and prompt tokens.
    #[arbitrary(with = crate::arbitrary_util::arbitrary_u64)]
    pub total_tokens: u64,
    /// Breakdown of completion tokens (reasoning, audio, etc.) if available.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub completion_tokens_details: Option<CompletionTokensDetails>,
    /// Breakdown of prompt tokens (cached, audio, etc.) if available.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub prompt_tokens_details: Option<PromptTokensDetails>,
    /// Cost charged by ObjectiveAI for this request.
    #[serde(deserialize_with = "crate::serde_util::decimal")]
    #[schemars(with = "f64")]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_rust_decimal)]
    pub cost: rust_decimal::Decimal,
    /// Breakdown of upstream and upstream_upstream costs if available.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub cost_details: Option<CostDetails>,
    /// Total cost including upstream provider charges. Only differs from `cost`
    /// when using BYOK (Bring Your Own Key).
    #[serde(deserialize_with = "crate::serde_util::decimal")]
    #[schemars(with = "f64")]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_rust_decimal)]
    pub total_cost: rust_decimal::Decimal,
}

impl Usage {
    /// Returns `true` if any usage metrics are non-zero.
    pub fn any_usage(&self) -> bool {
        self.completion_tokens > 0
            || self.prompt_tokens > 0
            || self.total_tokens > 0
            || self
                .completion_tokens_details
                .as_ref()
                .is_some_and(CompletionTokensDetails::any_usage)
            || self
                .prompt_tokens_details
                .as_ref()
                .is_some_and(PromptTokensDetails::any_usage)
            || self.cost > rust_decimal::Decimal::ZERO
            || self
                .cost_details
                .as_ref()
                .is_some_and(CostDetails::any_usage)
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

    /// Appends usage from an upstream usage response.
    pub fn push_upstream_usage(&mut self, other: &UpstreamUsage) {
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
