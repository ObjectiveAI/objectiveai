//! Usage statistics from OpenRouter responses.

use diverge_provider_sdk::endpoints::agentic_loop::run::server::response;
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
        Option<super::CompletionTokensDetails>,
    /// Detailed breakdown of prompt tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens_details:
        Option<super::PromptTokensDetails>,
    /// Cost charged by OpenRouter for this request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<rust_decimal::Decimal>,
    /// Detailed cost breakdown including upstream provider costs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_details: Option<CostDetails>,
}

/// Detailed cost breakdown from OpenRouter.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CostDetails {
    /// Cost charged by the upstream inference provider.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upstream_inference_cost: Option<rust_decimal::Decimal>,
}

impl Usage {
    /// Append this usage as a usage chunk: the three token counts.
    /// The detail breakdowns and the cost fields have no home in the
    /// chunk vocabulary and are dropped.
    pub fn into_chunks(self, chunks: &mut Vec<response::AgenticLoopChunk>) {
        chunks.push(response::AgenticLoopChunk::Usage(response::UsageChunk {
            r#type: Default::default(),
            completion_tokens: self.completion_tokens,
            prompt_tokens: self.prompt_tokens,
            total_tokens: self.total_tokens,
            meta: None,
        }));
    }
}
