//! The ONE error type for every [`super::Client`] surface — the
//! listeners' former per-module enums folded together with the plain
//! HTTP calls' status mapping.

/// Any failure across the daemon client's surfaces: opening an SSE
/// stream, a plain HTTP call, or decoding what came back.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The request could not be turned into an SSE stream (builder
    /// rejected it).
    #[error("connect daemon sse: {0}")]
    Connect(#[from] reqwest_eventsource::CannotCloneRequestError),
    /// The SSE stream failed to OPEN — transport failure or a
    /// non-success status (401 surfaces here as `InvalidStatusCode`).
    #[error("open daemon sse: {0}")]
    Open(#[from] reqwest_eventsource::Error),
    /// The stream ended before it ever opened.
    #[error("daemon sse closed before opening")]
    Closed,
    /// A plain HTTP call failed in transport (or the client failed to
    /// build).
    #[error("daemon http: {0}")]
    Http(#[from] reqwest::Error),
    /// No such resource (404) — an unknown/withdrawn channel, an
    /// unknown plugin tag.
    #[error("not found")]
    NotFound,
    /// The channel offer was already accepted by someone else (409).
    #[error("already accepted")]
    AlreadyAccepted,
    /// The daemon refused the credentials (401).
    #[error("unauthorized")]
    Unauthorized,
    /// Any other non-success status; carries the response body text.
    #[error("daemon status {status}: {body}")]
    Status {
        status: reqwest::StatusCode,
        body: String,
    },
    /// A response body wasn't the expected shape.
    #[error("daemon body parse: {0}")]
    Parse(#[from] serde_json::Error),
}

/// The `/execute` surface's error — the [`super::Client`]'s
/// [`CommandExecutor`](crate::cli::command::CommandExecutor)
/// associated error type. Separate from [`Error`]: execute streams
/// carry structured CLI errors ([`crate::cli::Error`]) that the
/// listener surfaces never produce.
#[cfg(feature = "cli-executor")]
#[derive(Debug, thiserror::Error)]
pub enum ExecuteError {
    /// Building/sending the request, or connecting, failed.
    #[error("connect daemon execute: {0}")]
    Connect(reqwest::Error),
    /// The daemon answered a non-success status (e.g. `401` on a
    /// missing/invalid signature).
    #[error("daemon execute http {0}: {1}")]
    Http(reqwest::StatusCode, String),
    /// The SSE response body failed mid-stream.
    #[error("daemon execute sse stream: {0}")]
    Sse(String),
    /// Serializing the request or decoding a response event failed.
    #[error("decode daemon execute message: {0}")]
    Json(serde_json::Error),
    /// Structured error emitted by the daemon on the stream.
    #[error("{0}")]
    Cli(crate::cli::Error),
    /// `execute_one` was called but the stream produced no items.
    #[error("daemon execute stream produced no items")]
    Empty,
}
