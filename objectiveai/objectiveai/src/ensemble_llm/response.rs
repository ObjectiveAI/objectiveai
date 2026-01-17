//! Response types for Ensemble LLM API endpoints.

use serde::{Deserialize, Serialize};

/// Response containing a list of Ensemble LLMs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListEnsembleLlm {
    /// The list of Ensemble LLM summaries.
    pub data: Vec<ListEnsembleLlmItem>,
}

/// Summary information for a listed Ensemble LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListEnsembleLlmItem {
    /// The unique content-addressed ID of the Ensemble LLM.
    pub id: String,
}

/// Response containing a single Ensemble LLM with creation timestamp.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetEnsembleLlm {
    /// Unix timestamp when this Ensemble LLM was first used.
    pub created: u64,
    /// The Ensemble LLM definition.
    #[serde(flatten)]
    pub inner: super::EnsembleLlm,
}

/// Usage statistics for an Ensemble LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageEnsembleLlm {
    /// Total number of requests made with this Ensemble LLM.
    pub requests: u64,
    /// Total completion tokens generated.
    pub completion_tokens: u64,
    /// Total prompt tokens processed.
    pub prompt_tokens: u64,
    /// Total cost incurred.
    pub total_cost: rust_decimal::Decimal,
}
