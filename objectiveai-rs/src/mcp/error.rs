//! MCP client errors.

/// Walk the `Error::source` chain of any `std::error::Error` and join
/// every level's `Display` with `: ` so the bottom-most cause (e.g. an
/// I/O `deadline has elapsed` deep under a `reqwest::Error`) actually
/// reaches the user instead of being hidden behind a generic outer
/// wrapper.
fn fmt_error_chain(err: &dyn std::error::Error) -> String {
    let mut out = err.to_string();
    let mut current = err.source();
    while let Some(source) = current {
        let s = source.to_string();
        // Skip duplicate levels — `reqwest::Error::Display` sometimes
        // repeats its own source, which would produce noise like
        // `... : foo: foo`.
        if !out.ends_with(&s) {
            out.push_str(": ");
            out.push_str(&s);
        }
        current = source.source();
    }
    out
}

/// Errors that can occur during MCP operations.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Failed to connect to the MCP server.
    #[error("connection error to {url}: {}", fmt_error_chain(source))]
    Connection {
        /// The URL the failing request was targeting.
        url: String,
        /// The underlying reqwest error.
        source: reqwest::Error,
    },
    /// HTTP request failed (post-connect).
    #[error("request error to {url}: {}", fmt_error_chain(source))]
    Request {
        /// The URL the failing request was targeting.
        url: String,
        /// The underlying reqwest error.
        source: reqwest::Error,
    },
    /// Server returned a non-success HTTP status code.
    #[error("bad status from {url} ({code}): {body}")]
    BadStatus {
        /// The URL the failing request was targeting.
        url: String,
        /// The HTTP status code received.
        code: reqwest::StatusCode,
        /// The response body.
        body: String,
    },
    /// The server returned a JSON-RPC error.
    #[error("json-rpc error from {url} ({code}): {message}{}", data.as_ref().map(|d| format!("; data: {d}")).unwrap_or_default())]
    JsonRpc {
        /// The URL the failing request was targeting.
        url: String,
        /// The JSON-RPC error code.
        code: i64,
        /// The error message.
        message: String,
        /// Optional additional error data.
        data: Option<serde_json::Value>,
    },
    /// The session expired (server returned 404).
    #[error("session expired at {url}")]
    SessionExpired {
        /// The URL whose session was expired.
        url: String,
    },
    /// The server did not return a session ID on initialization.
    #[error("server did not return Mcp-Session-Id header at {url}; body: {body}")]
    NoSessionId {
        /// The URL we attempted to initialize against.
        url: String,
        /// The response body the server returned, truncated to a
        /// reasonable preview length. Often carries a JSON-RPC error
        /// describing why the session wasn't created (e.g., upstream
        /// connect failed for a specific URL).
        body: String,
    },
    /// Authorization required but not provided for this MCP server URL.
    #[error("missing authorization for MCP server: {0}")]
    MissingAuthorization(String),
    /// The server returned a body that wasn't decodable as JSON or SSE.
    #[error("malformed JSON-RPC response from {url}: {message}")]
    MalformedResponse {
        /// The URL that produced the unparseable response.
        url: String,
        /// What was wrong with the body, including a preview.
        message: String,
    },
}
