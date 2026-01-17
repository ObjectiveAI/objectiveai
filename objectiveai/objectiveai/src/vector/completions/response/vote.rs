//! Vote type representing a single LLM's selection in a vector completion.

use serde::{Deserialize, Serialize};

/// A single LLM's vote in a vector completion.
///
/// Each LLM in the ensemble produces a vote indicating which response(s) it
/// selected. Votes are weighted according to the profile and combined to
/// produce the final scores.
///
/// # Vote Format
///
/// The `vote` field is a vector of decimals corresponding to the responses
/// in the request. Typically one element is 1.0 and the rest are 0.0 (discrete
/// selection), but when `top_logprobs` is used, votes may be probability
/// distributions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    // --- Identifiers ---

    /// The model that produced this vote (e.g., `"openai/gpt-4o"`).
    pub model: String,
    /// Index of the LLM configuration within the ensemble.
    pub ensemble_index: u64,
    /// Flattened index accounting for LLM counts in the ensemble.
    pub flat_ensemble_index: u64,
    /// Content hash of the request messages (for caching/deduplication).
    pub prompt_id: String,
    /// Content hash of the request tools, if any.
    pub tools_id: Option<String>,
    /// Content hashes of each response option in the request.
    pub responses_ids: Vec<String>,

    // --- Vote data ---

    /// The vote distribution. Each index corresponds to a response from the
    /// request. Typically one element is 1.0 (selected) and the rest are 0.0.
    pub vote: Vec<rust_decimal::Decimal>,

    /// The weight applied to this vote when computing final scores.
    pub weight: rust_decimal::Decimal,

    // --- Source flags ---

    /// If true, this vote was reused from a previous request via the `retry`
    /// parameter. All fields reflect the original request's values.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry: Option<bool>,

    /// If true, this vote was retrieved from cache rather than generated fresh.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_cache: Option<bool>,

    /// If true, this vote was randomly generated (for testing/simulation).
    /// Mutually exclusive with `from_cache` and `retry`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_rng: Option<bool>,

    // --- Internal ---

    /// Internal index for correlating with completions. Not serialized.
    #[serde(skip)]
    pub completion_index: Option<u64>,
}
