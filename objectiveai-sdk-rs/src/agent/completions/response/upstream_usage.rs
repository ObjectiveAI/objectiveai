//! Token usage and cost information from an upstream provider.

use super::{CompletionTokensDetails, CostDetails, PromptTokensDetails};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Token usage and cost information from an upstream provider.
///
/// This is the per-assistant-response usage yielded by upstream clients.
/// It includes upstream-specific fields like `cost_multiplier` and `is_byok`.
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
#[schemars(rename = "agent.completions.response.UpstreamUsage")]
pub struct UpstreamUsage {
    /// Number of tokens in the completion.
    #[arbitrary(with = crate::arbitrary_util::arbitrary_u64)]
    pub completion_tokens: u64,
    /// Number of tokens in the prompt.
    #[arbitrary(with = crate::arbitrary_util::arbitrary_u64)]
    pub prompt_tokens: u64,
    /// Total tokens (prompt + completion).
    #[arbitrary(with = crate::arbitrary_util::arbitrary_u64)]
    pub total_tokens: u64,
    /// Detailed breakdown of completion tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub completion_tokens_details: Option<CompletionTokensDetails>,
    /// Detailed breakdown of prompt tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub prompt_tokens_details: Option<PromptTokensDetails>,
    /// The cost charged by ObjectiveAI for this request.
    #[serde(deserialize_with = "crate::serde_util::decimal")]
    #[schemars(with = "f64")]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_rust_decimal)]
    pub cost: rust_decimal::Decimal,
    /// Detailed cost breakdown.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub cost_details: Option<CostDetails>,
    /// Total cost including ObjectiveAI's charge plus all upstream charges.
    /// For BYOK requests, ObjectiveAI only charges the cost_multiplier difference,
    /// but total_cost still includes what the upstream provider charged.
    #[serde(deserialize_with = "crate::serde_util::decimal")]
    #[schemars(with = "f64")]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_rust_decimal)]
    pub total_cost: rust_decimal::Decimal,
    /// The multiplier applied to compute ObjectiveAI's charge.
    #[serde(deserialize_with = "crate::serde_util::decimal")]
    #[schemars(with = "f64")]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_rust_decimal)]
    pub cost_multiplier: rust_decimal::Decimal,
    /// Whether this request used Bring Your Own Key (BYOK).
    pub is_byok: bool,
}

impl UpstreamUsage {
    /// Appends usage statistics from another instance.
    pub fn push(&mut self, other: &UpstreamUsage) {
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
