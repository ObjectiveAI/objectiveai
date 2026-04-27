//! MCP client errors.

/// Errors that can occur during MCP operations.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Failed to connect to the MCP server.
    #[error("connection error: {0}")]
    Connection(reqwest::Error),
    /// HTTP request failed.
    #[error("request error: {0}")]
    Request(reqwest::Error),
    /// Server returned a non-success HTTP status code.
    #[error("bad status {code}: {body}")]
    BadStatus {
        /// The HTTP status code received.
        code: reqwest::StatusCode,
        /// The response body.
        body: String,
    },
    /// The server returned a JSON-RPC error.
    #[error("json-rpc error {code}: {message}")]
    JsonRpc {
        /// The JSON-RPC error code.
        code: i64,
        /// The error message.
        message: String,
        /// Optional additional error data.
        data: Option<serde_json::Value>,
    },
    /// The session expired (server returned 404).
    #[error("session expired")]
    SessionExpired,
    /// The server did not return a session ID on initialization.
    #[error("server did not return Mcp-Session-Id header")]
    NoSessionId,
    /// Authorization required but not provided for this MCP server URL.
    #[error("missing authorization for MCP server: {0}")]
    MissingAuthorization(String),
}
