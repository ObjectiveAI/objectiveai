//! Request body types for function executions.

use crate::{chat, functions};
use serde::{Deserialize, Serialize};

/// Request body for inline Function with inline Profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionInlineProfileInlineRequestBody {
    /// The inline Function definition.
    pub function: functions::InlineFunction,
    /// The inline Profile definition.
    pub profile: functions::InlineProfile,
    /// Common execution parameters.
    #[serde(flatten)]
    pub base: FunctionRemoteProfileRemoteRequestBody,
}

/// Request body for inline Function with remote Profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionInlineProfileRemoteRequestBody {
    /// The inline Function definition.
    pub function: functions::InlineFunction,
    /// Common execution parameters.
    #[serde(flatten)]
    pub base: FunctionRemoteProfileRemoteRequestBody,
}

/// Request body for remote Function with inline Profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionRemoteProfileInlineRequestBody {
    /// The inline Profile definition.
    pub profile: functions::InlineProfile,
    /// Common execution parameters.
    #[serde(flatten)]
    pub base: FunctionRemoteProfileRemoteRequestBody,
}

/// Base request body with common execution parameters.
///
/// Used directly for remote Function + remote Profile, or flattened into
/// other request body types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionRemoteProfileRemoteRequestBody {
    // --- Caching and retry options ---

    /// If present, reuses votes from a previous execution with this token.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_token: Option<String>,
    /// If true, uses cached votes when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_cache: Option<bool>,
    /// If true, remaining votes are generated randomly (for testing/simulation).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_rng: Option<bool>,

    // --- Reasoning configuration ---

    /// Reasoning summary configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<super::Reasoning>,

    // --- Core configuration ---

    /// The input data to pass to the Function.
    pub input: functions::expression::Input,
    /// Provider routing preferences.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<chat::completions::request::Provider>,
    /// Random seed for deterministic results.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,
    /// Whether to stream the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

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
