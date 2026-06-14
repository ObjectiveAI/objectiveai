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

    /// Two attempts failed (e.g. the GitHub raw + Contents-API fallback).
    #[error("multiple errors: {0}, {1}")]
    MultipleErrors(Box<HttpError>, Box<HttpError>),

    /// The API returned a structured error response.
    #[error(transparent)]
    ApiError(#[from] error::ResponseError),

    /// Failed to upgrade the request to a WebSocket. Used by the
    /// `send_streaming_ws` path before any frames have flowed.
    #[error("websocket upgrade failed: {0}")]
    WsConnect(#[from] tokio_tungstenite::tungstenite::Error),

    /// Failed to serialize the notify request body to JSON.
    #[error("notify serialize: {0}")]
    NotifySerialize(serde_json::Error),

    /// Failed to write the notify frame to the WebSocket sink.
    #[error("notify send: {0}")]
    NotifySend(tokio_tungstenite::tungstenite::Error),

    /// The WebSocket closed before the matching `client_response`
    /// arrived. Either the server hung up or the demux task exited.
    #[error("notify channel closed before response arrived")]
    NotifyChannelClosed,

    /// The server replied to a notify with `client_response::Error`.
    /// The most common cause is the notify's `response_id` not
    /// matching any agent completion this WS produced.
    #[error("notify rejected: code={code} message={message}")]
    NotifyRejected {
        code: u16,
        message: serde_json::Value,
    },
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
            HttpError::MultipleErrors(e1, e2) => {
                let s2 = e2.status();
                if s2 != 500 { s2 } else { e1.status() }
            }
            HttpError::ApiError(e) => e.status(),
            HttpError::WsConnect(_) => 500,
            HttpError::NotifySerialize(_) => 500,
            HttpError::NotifySend(_) => 500,
            HttpError::NotifyChannelClosed => 500,
            HttpError::NotifyRejected { code, .. } => *code,
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
                HttpError::MultipleErrors(e1, e2) => serde_json::json!({
                    "kind": "multiple",
                    "error_1": e1.message(),
                    "error_2": e2.message(),
                }),
                HttpError::ApiError(e) => serde_json::json!({
                    "kind": "api_error",
                    "error": e.message(),
                }),
                HttpError::WsConnect(e) => serde_json::json!({
                    "kind": "ws_connect",
                    "error": e.to_string(),
                }),
                HttpError::NotifySerialize(e) => serde_json::json!({
                    "kind": "notify_serialize",
                    "error": e.to_string(),
                }),
                HttpError::NotifySend(e) => serde_json::json!({
                    "kind": "notify_send",
                    "error": e.to_string(),
                }),
                HttpError::NotifyChannelClosed => serde_json::json!({
                    "kind": "notify_channel_closed",
                }),
                HttpError::NotifyRejected { code, message } => serde_json::json!({
                    "kind": "notify_rejected",
                    "code": code,
                    "message": message,
                }),
            }
        }))
    }
}
