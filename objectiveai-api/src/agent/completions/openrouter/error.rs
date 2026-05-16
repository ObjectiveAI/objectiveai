//! Errors for OpenRouter agent completions.

use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Errors that can occur during OpenRouter agent completion streaming.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An MCP server returned an error when listing tools.
    #[error("MCP error ({url}): {error}")]
    Mcp {
        /// The URL of the MCP server that errored.
        url: String,
        /// The underlying MCP error.
        error: Arc<objectiveai_sdk::mcp::Error>,
    },

    /// Error returned by the OpenRouter provider.
    #[error("provider error: {0}")]
    OpenRouterProviderError(#[from] OpenRouterProviderError),

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

    /// The upstream produced no chunks.
    #[error("empty stream")]
    EmptyStream,

    /// Tools are not allowed but response format requires a tool call.
    #[error("tools not allowed but response format requires a tool call")]
    ToolsNotAllowedWithRequiredToolCall,

    /// No API key available (neither BYOK nor server-side).
    #[error("missing API key: no BYOK provided and no server-side OpenRouter API key configured")]
    MissingApiKey,
}

impl objectiveai_sdk::error::StatusError for Error {
    fn status(&self) -> u16 {
        match self {
            Self::Mcp { .. } => 502,
            Self::OpenRouterProviderError(e) => e.status(),
            Self::DeserializationError(_) => 500,
            Self::BadStatus { code, .. } => code.as_u16(),
            Self::StreamError(reqwest_eventsource::Error::InvalidStatusCode(code, _)) => {
                code.as_u16()
            }
            Self::StreamError(reqwest_eventsource::Error::Transport(e)) => {
                e.status().map(|s| s.as_u16()).unwrap_or(500)
            }
            Self::StreamError(_) => 500,
            Self::EmptyStream => 500,
            Self::ToolsNotAllowedWithRequiredToolCall => 400,
            Self::MissingApiKey => 401,
        }
    }

    fn message(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "kind": "openrouter",
            "error": match self {
                Self::Mcp { url, error } => serde_json::json!({
                    "kind": "mcp",
                    "url": url,
                    "error": error.to_string(),
                }),
                Self::OpenRouterProviderError(e) => serde_json::json!({
                    "kind": "provider_error",
                    "error": e.message(),
                }),
                Self::DeserializationError(e) => serde_json::json!({
                    "kind": "deserialization",
                    "error": e.to_string(),
                }),
                Self::BadStatus { body, .. } => serde_json::json!({
                    "kind": "bad_status",
                    "error": body,
                }),
                Self::StreamError(e) => serde_json::json!({
                    "kind": "stream_error",
                    "error": e.to_string(),
                }),
                Self::EmptyStream => serde_json::json!({
                    "kind": "empty_stream",
                    "error": "received an empty stream",
                }),
                Self::ToolsNotAllowedWithRequiredToolCall => serde_json::json!({
                    "kind": "tools_not_allowed",
                    "error": "tools not allowed but response format requires a tool call",
                }),
                Self::MissingApiKey => serde_json::json!({
                    "kind": "missing_api_key",
                    "error": "no BYOK provided and no server-side OpenRouter API key configured",
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

impl objectiveai_sdk::error::StatusError for OpenRouterProviderError {
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

impl objectiveai_sdk::error::StatusError for OpenRouterProviderErrorInner {
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
