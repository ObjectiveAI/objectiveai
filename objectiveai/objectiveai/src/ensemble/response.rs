//! Response types for Ensemble API endpoints.

use serde::{Deserialize, Serialize};

/// Response containing a list of Ensembles.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListEnsemble {
    /// The list of Ensemble summaries.
    pub data: Vec<ListEnsembleItem>,
}

/// Summary information for a listed Ensemble.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListEnsembleItem {
    /// The unique content-addressed ID of the Ensemble.
    pub id: String,
}

/// Response containing a single Ensemble with creation timestamp.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetEnsemble {
    /// Unix timestamp when this Ensemble was first used.
    pub created: u64,
    /// The Ensemble definition.
    #[serde(flatten)]
    pub inner: super::Ensemble,
}

/// Usage statistics for an Ensemble.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageEnsemble {
    /// Total number of requests made with this Ensemble.
    pub requests: u64,
    /// Total completion tokens generated across all LLMs.
    pub completion_tokens: u64,
    /// Total prompt tokens processed across all LLMs.
    pub prompt_tokens: u64,
    /// Total cost incurred.
    pub total_cost: rust_decimal::Decimal,
}
