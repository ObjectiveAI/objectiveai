//! Vector completion request parameters.

use crate::chat;
use serde::{Deserialize, Serialize};

/// Parameters for creating a vector completion.
///
/// Vector completions run multiple chat completions (one per LLM in the
/// ensemble), force each to vote for one of the predefined responses, and
/// combine votes using the provided profile weights to produce final scores.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorCompletionCreateParams {
    // --- Caching and retry options ---

    /// If present, reuses votes from a previous request with this ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry: Option<String>,
    /// If true, uses cached votes when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_cache: Option<bool>,
    /// If true, remaining votes are generated randomly (for testing/simulation).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_rng: Option<bool>,

    // --- Core configuration ---

    /// The conversation messages (the prompt).
    pub messages: Vec<chat::completions::request::Message>,
    /// Provider routing preferences.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<chat::completions::request::Provider>,
    /// The Ensemble of LLMs to use.
    pub ensemble: super::Ensemble,
    /// The weights for each LLM in the ensemble. Must have the same length as
    /// the total LLM count in the ensemble.
    pub profile: Vec<rust_decimal::Decimal>,
    /// Random seed for deterministic results.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,
    /// Whether to stream the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    /// Tools available (read-only context, not callable in vector completions).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<chat::completions::request::Tool>>,
    /// The possible responses the LLMs can vote for.
    pub responses: Vec<chat::completions::request::RichContent>,

    // --- Retry configuration ---

    /// Maximum elapsed time (ms) for exponential backoff retries.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backoff_max_elapsed_time: Option<u64>,
    /// Timeout (ms) for receiving the first chunk of a streaming response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_chunk_timeout: Option<u64>,
    /// Timeout (ms) between subsequent chunks of a streaming response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub other_chunk_timeout: Option<u64>,
}
