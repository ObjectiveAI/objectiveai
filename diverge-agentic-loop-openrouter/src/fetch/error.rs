//! What can go wrong between this container and OpenRouter.

use serde::{Deserialize, Serialize};

/// A fetch that failed, or a stream that did.
///
/// Ported from the api crate's OpenRouter error, trimmed to what
/// exists here: no MCP arm (tools ride the proxy, not this call) and
/// no response-format arm (the field is gone).
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Error returned by the OpenRouter provider, as an SSE data
    /// event.
    #[error("provider error: {0}")]
    Provider(#[from] ProviderError),

    /// Failed to deserialize a response from OpenRouter.
    #[error("deserialization error: {0}")]
    Deserialization(#[from] serde_path_to_error::Error<serde_json::Error>),

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
    Stream(#[from] reqwest_eventsource::Error),

    /// The upstream produced no chunks.
    #[error("empty stream")]
    EmptyStream,
}

/// Error response from OpenRouter containing provider error details.
#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
#[error("{}", &serde_json::to_string(self).unwrap_or_default())]
pub struct ProviderError {
    /// The inner error details from the provider.
    pub error: ProviderErrorInner,
    /// Optional user ID associated with the error.
    pub user_id: Option<String>,
}

/// Inner error details from the OpenRouter provider.
#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
#[error("{}", &serde_json::to_string(self).unwrap_or_default())]
pub struct ProviderErrorInner {
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

impl Error {
    /// The failure as JSON, in the shape the api crate reported it.
    pub fn message(&self) -> serde_json::Value {
        serde_json::json!({
            "kind": "openrouter",
            "error": match self {
                Error::Provider(error) => serde_json::json!({
                    "kind": "provider_error",
                    "error": {
                        "kind": "provider",
                        "message": error.error.message,
                        "metadata": error.error.metadata,
                    },
                }),
                Error::Deserialization(error) => serde_json::json!({
                    "kind": "deserialization",
                    "error": error.to_string(),
                }),
                Error::BadStatus { body, .. } => serde_json::json!({
                    "kind": "bad_status",
                    "error": body,
                }),
                Error::Stream(error) => serde_json::json!({
                    "kind": "stream_error",
                    "error": error.to_string(),
                }),
                Error::EmptyStream => serde_json::json!({
                    "kind": "empty_stream",
                    "error": "received an empty stream",
                }),
            },
        })
    }
}
