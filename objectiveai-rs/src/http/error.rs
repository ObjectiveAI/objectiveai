//! HTTP error types.

use crate::error;

/// Errors that can occur during HTTP operations.
#[derive(thiserror::Error, Debug)]
pub enum HttpError {
    /// Failed to deserialize the response body.
    ///
    /// Includes path information to help identify which field caused the error.
    #[error("deserialization error: {0}")]
    DeserializationError(#[from] serde_path_to_error::Error<serde_json::Error>),

    /// The server returned a non-success HTTP status code.
    #[error("received bad status code: {code}, body: {body}")]
    BadStatus {
        /// The HTTP status code (e.g., 400, 401, 500).
        code: reqwest::StatusCode,
        /// Response body, parsed as JSON if possible, otherwise as a string.
        body: serde_json::Value,
    },

    /// Error occurred while reading from an SSE stream.
    #[error("error fetching stream: {0}")]
    StreamError(#[from] reqwest_eventsource::Error),

    /// Failed to build the HTTP request.
    #[error("request error: {0}")]
    RequestError(reqwest::Error),

    /// Failed to establish a streaming connection.
    ///
    /// Occurs when the request cannot be cloned for SSE retry logic.
    #[error("streaming request error: {0}")]
    StreamingRequestError(#[from] reqwest_eventsource::CannotCloneRequestError),

    /// General HTTP transport error (network, timeout, etc.).
    #[error("http error: {0}")]
    HttpError(reqwest::Error),

    /// The API returned a structured error response.
    #[error(transparent)]
    ApiError(#[from] error::ResponseError),
}

impl error::StatusError for HttpError {
    fn status(&self) -> u16 {
        match self {
            HttpError::DeserializationError(_) => 500,
            HttpError::BadStatus { code, .. } => code.as_u16(),
            HttpError::StreamError(reqwest_eventsource::Error::Transport(
                e,
            )) => e.status().map(|s| s.as_u16()).unwrap_or(500),
            HttpError::StreamError(
                reqwest_eventsource::Error::InvalidStatusCode(code, _),
            ) => code.as_u16(),
            HttpError::StreamError(_) => 500,
            HttpError::RequestError(e) => {
                e.status().map(|s| s.as_u16()).unwrap_or(500)
            }
            HttpError::StreamingRequestError(_) => 500,
            HttpError::HttpError(e) => {
                e.status().map(|s| s.as_u16()).unwrap_or(500)
            }
            HttpError::ApiError(e) => e.status(),
        }
    }

    fn message(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "kind": "objectiveai_client",
            "error": match self {
                HttpError::DeserializationError(e) => serde_json::json!({
                    "kind": "deserialization",
                    "error": e.to_string(),
                }),
                HttpError::BadStatus { body, .. } => serde_json::json!({
                    "kind": "bad_status",
                    "error": body,
                }),
                HttpError::StreamError(e) => serde_json::json!({
                    "kind": "stream_error",
                    "error": e.to_string(),
                }),
                HttpError::RequestError(e) => serde_json::json!({
                    "kind": "request_error",
                    "error": e.to_string(),
                }),
                HttpError::StreamingRequestError(e) => serde_json::json!({
                    "kind": "streaming_request_error",
                    "error": e.to_string(),
                }),
                HttpError::HttpError(e) => serde_json::json!({
                    "kind": "http_error",
                    "error": e.to_string(),
                }),
                HttpError::ApiError(e) => serde_json::json!({
                    "kind": "api_error",
                    "error": e.message(),
                }),
            }
        }))
    }
}
