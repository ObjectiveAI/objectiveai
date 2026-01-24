//! Error types for OpenRouter provider operations.

use serde::{Deserialize, Serialize};

/// Errors that can occur when communicating with the OpenRouter provider.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Error returned by the OpenRouter provider.
    #[error("provider error: {0}")]
    OpenRouterProviderError(#[from] OpenRouterProviderError),
    /// The provider returned an empty stream with no chunks.
    #[error("received an empty stream")]
    EmptyStream,
    /// Failed to deserialize a response from OpenRouter.
    #[error("deserialization error: {0}")]
    DeserializationError(#[from] serde_path_to_error::Error<serde_json::Error>),
    /// The provider returned a non-success HTTP status code.
    #[error("received bad status code: {code}, body: {body}")]
    BadStatus {
        /// The HTTP status code received.
        code: reqwest::StatusCode,
        /// The response body, parsed as JSON if possible.
        body: serde_json::Value,
    },
    /// Error occurred while fetching or processing the SSE stream.
    #[error("error fetching stream: {0}")]
    StreamError(#[from] reqwest_eventsource::Error),
    /// The stream timed out waiting for chunks.
    #[error("error fetching stream: timeout")]
    StreamTimeout,
    /// Failed to fetch an Ensemble LLM definition.
    #[error("fetch Ensemble LLM error: {0}")]
    FetchEnsembleLlm(objectiveai::error::ResponseError),
    /// The requested Ensemble LLM was not found.
    #[error("Ensemble LLM not found")]
    EnsembleLlmNotFound,
    /// The user has insufficient credits to complete the request.
    #[error("insufficient credits")]
    InsufficientCredits,
    /// The Ensemble LLM configuration is invalid.
    #[error("invalid Ensemble LLM: {0}")]
    InvalidEnsembleLlm(String),
}

impl objectiveai::error::StatusError for Error {
    fn status(&self) -> u16 {
        match self {
            Error::OpenRouterProviderError(e) => e.status(),
            Error::EmptyStream => 500,
            Error::DeserializationError(_) => 500,
            Error::BadStatus { code, .. } => code.as_u16(),
            Error::StreamError(reqwest_eventsource::Error::Transport(e)) => {
                e.status().map(|s| s.as_u16()).unwrap_or(500)
            }
            Error::StreamError(reqwest_eventsource::Error::InvalidStatusCode(code, _)) => {
                code.as_u16()
            }
            Error::StreamError(_) => 500,
            Error::StreamTimeout => 500,
            Error::FetchEnsembleLlm(e) => e.status(),
            Error::InsufficientCredits => 402,
            Error::InvalidEnsembleLlm(_) => 400,
            Error::EnsembleLlmNotFound => 404,
        }
    }

    fn message(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "kind": "openrouter",
            "error": match self {
                Error::OpenRouterProviderError(e) => serde_json::json!({
                    "kind": "provider_error",
                    "error": e.message(),
                }),
                Error::EmptyStream => serde_json::json!({
                    "kind": "empty_stream",
                    "error": "received an empty stream",
                }),
                Error::DeserializationError(e) => serde_json::json!({
                    "kind": "deserialization",
                    "error": e.to_string(),
                }),
                Error::BadStatus { body, .. } => serde_json::json!({
                    "kind": "bad_status",
                    "error": body,
                }),
                Error::StreamError(e) => serde_json::json!({
                    "kind": "stream_error",
                    "error": e.to_string(),
                }),
                Error::StreamTimeout => serde_json::json!({
                    "kind": "stream_timeout",
                    "error": "error fetching stream: timeout",
                }),
                Error::FetchEnsembleLlm(e) => serde_json::json!({
                    "kind": "fetch_ensemble_llm",
                    "error": e.message(),
                }),
                Error::InsufficientCredits => serde_json::json!({
                    "kind": "insufficient_credits",
                    "error": "the user has insufficient credits",
                }),
                Error::InvalidEnsembleLlm(msg) => serde_json::json!({
                    "kind": "invalid_ensemble_llm",
                    "error": msg,
                }),
                Error::EnsembleLlmNotFound => serde_json::json!({
                    "kind": "ensemble_llm_not_found",
                    "error": "Ensemble LLM not found",
                }),
            },
        }))
    }
}

/// Error response from OpenRouter containing provider error details.
#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
#[error("{}", &serde_json::to_string(self).unwrap_or_default())]
pub struct OpenRouterProviderError {
    /// The inner error details from the provider.
    pub error: OpenRouterProviderErrorInner,
    /// Optional user ID associated with the error.
    pub user_id: Option<String>,
}

impl objectiveai::error::StatusError for OpenRouterProviderError {
    fn status(&self) -> u16 {
        self.error.status()
    }

    fn message(&self) -> Option<serde_json::Value> {
        self.error.message()
    }
}

/// Inner error details from the OpenRouter provider.
#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
#[error("{}", &serde_json::to_string(self).unwrap_or_default())]
pub struct OpenRouterProviderErrorInner {
    /// The HTTP status code from the provider, if available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<u16>,
    /// The error message from the provider.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<serde_json::Value>,
    /// Additional metadata about the error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl objectiveai::error::StatusError for OpenRouterProviderErrorInner {
    fn status(&self) -> u16 {
        self.code
            .unwrap_or(reqwest::StatusCode::INTERNAL_SERVER_ERROR.as_u16())
    }

    fn message(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "kind": "provider",
            "message": self.message,
            "metadata": self.metadata,
        }))
    }
}
