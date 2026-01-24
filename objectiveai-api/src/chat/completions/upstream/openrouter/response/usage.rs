//! Usage statistics from OpenRouter responses.

use serde::{Deserialize, Serialize};

/// Token usage and cost statistics from OpenRouter.
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
    pub completion_tokens_details:
        Option<objectiveai::chat::completions::response::CompletionTokensDetails>,
    /// Detailed breakdown of prompt tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens_details:
        Option<objectiveai::chat::completions::response::PromptTokensDetails>,
    /// Cost charged by OpenRouter for this request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<rust_decimal::Decimal>,
    /// Detailed cost breakdown including upstream provider costs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_details: Option<CostDetails>,
}

impl Usage {
    /// Transforms upstream usage into downstream format with cost calculations.
    ///
    /// Applies the cost multiplier and separates BYOK costs from ObjectiveAI costs.
    pub fn into_downstream(
        self,
        is_byok: bool,
        cost_multiplier: rust_decimal::Decimal,
    ) -> objectiveai::chat::completions::response::Usage {
        let upstream_inference_cost = self.cost.unwrap_or_default();
        let upstream_upstream_inference_cost = self
            .cost_details
            .unwrap_or_default()
            .upstream_inference_cost
            .unwrap_or_default();
        let upstream_total_cost = upstream_inference_cost + upstream_upstream_inference_cost;
        let total_cost = upstream_total_cost * cost_multiplier;
        let (cost, cost_details, total_cost) = if is_byok {
            (
                total_cost - upstream_total_cost,
                Some(objectiveai::chat::completions::response::CostDetails {
                    upstream_inference_cost,
                    upstream_upstream_inference_cost,
                }),
                total_cost,
            )
        } else {
            (total_cost, None, total_cost)
        };
        objectiveai::chat::completions::response::Usage {
            completion_tokens: self.completion_tokens,
            prompt_tokens: self.prompt_tokens,
            total_tokens: self.total_tokens,
            completion_tokens_details: self.completion_tokens_details,
            prompt_tokens_details: self.prompt_tokens_details,
            cost,
            cost_details,
            total_cost,
            cost_multiplier,
            is_byok,
        }
    }

    /// Accumulates usage from another Usage struct.
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
        objectiveai::chat::completions::response::util::push_option_decimal(
            &mut self.cost,
            &other.cost,
        );
        match (&mut self.cost_details, &other.cost_details) {
            (Some(self_cost_details), Some(other_cost_details)) => {
                self_cost_details.push(other_cost_details);
            }
            (None, Some(other_cost_details)) => {
                self.cost_details = Some(other_cost_details.clone());
            }
            _ => {}
        }
    }
}

/// Detailed cost breakdown from OpenRouter.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CostDetails {
    /// Cost charged by the upstream inference provider.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upstream_inference_cost: Option<rust_decimal::Decimal>,
}

impl CostDetails {
    /// Accumulates cost details from another CostDetails struct.
    pub fn push(&mut self, other: &CostDetails) {
        objectiveai::chat::completions::response::util::push_option_decimal(
            &mut self.upstream_inference_cost,
            &other.upstream_inference_cost,
        );
    }
}
