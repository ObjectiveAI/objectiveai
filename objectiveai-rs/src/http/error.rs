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
