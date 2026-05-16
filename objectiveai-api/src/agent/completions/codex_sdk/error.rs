use thiserror::Error;

/// Errors raised by the Codex SDK upstream client. The first five
/// variants mirror the `CodexSdkError` family in the Python SDK
/// (`errors.py`); the rest are orchestration errors raised on the Rust
/// side while spawning, reading, or validating against the runner
/// subprocess.
///
/// `AbortError` from the Python SDK is intentionally not represented:
/// Python's `AbortError` inherits from `Exception` directly (not
/// `CodexSdkError`), so it lives as a standalone struct in
/// [`super::AbortError`].
#[derive(Debug, Error)]
pub enum Error {
    // --- wire errors mirrored from the Python SDK ----------------------------
    /// A JSONL line could not be parsed into a [`super::ThreadEvent`].
    /// Mirrors `EventParseError` in `errors.py:12-13`.
    #[error("failed to parse thread event line: {0}")]
    EventParse(#[from] serde_json::Error),

    /// The codex subprocess exited with a non-zero status. The string is the
    /// captured stderr. Mirrors `CodexExecError` in `errors.py:16-17`.
    #[error("codex exec failed: {0}")]
    Exec(String),

    /// The runner emitted a `turn.failed` event; the inner string is the
    /// `error.message` payload. Mirrors `ThreadRunError` in `errors.py:8-9`.
    #[error("thread run error: {0}")]
    ThreadRun(String),

    /// Installing the codex CLI binary failed. Mirrors `CodexInstallError`
    /// in `errors.py:20-21`.
    #[error("codex install failed: {0}")]
    Install(String),

    /// Reading or writing Codex `auth.json` failed. Mirrors `CodexAuthError`
    /// in `errors.py:24-25`.
    #[error("codex auth failed: {0}")]
    Auth(String),

    // --- orchestration errors (Rust client side) -----------------------------
    #[error("rate limited")]
    RateLimit,

    #[error("invalid continuation: {0}")]
    InvalidContinuation(String),

    #[error("BYOK is not supported for Codex SDK")]
    InvalidByok,

    #[error("invalid messages: {0}")]
    InvalidMessages(String),

    #[error("unsupported response format")]
    UnsupportedResponseFormat,

    #[error("spawn error: {0}")]
    Spawn(String),

    #[error("IO error: {0}")]
    Io(String),

    #[error("JSON error: {0}")]
    Json(String),

    #[error("stderr: {0}")]
    Stderr(String),

    #[error("no output from subprocess")]
    NoOutput,

    #[error("Codex SDK does not support tools")]
    ToolsNotAllowed,

    #[error("Codex SDK is not enabled")]
    NotEnabled,

    #[error("image fetch failed: {0}")]
    ImageFetch(String),
}

impl objectiveai_sdk::error::StatusError for Error {
    fn status(&self) -> u16 {
        match self {
            Self::RateLimit => 429,
            Self::InvalidContinuation(_)
            | Self::InvalidByok
            | Self::InvalidMessages(_)
            | Self::UnsupportedResponseFormat
            | Self::ToolsNotAllowed
            | Self::NotEnabled => 400,
            Self::EventParse(_)
            | Self::Exec(_)
            | Self::ThreadRun(_)
            | Self::Install(_)
            | Self::Auth(_)
            | Self::Spawn(_)
            | Self::Io(_)
            | Self::Json(_)
            | Self::Stderr(_)
            | Self::NoOutput
            | Self::ImageFetch(_) => 500,
        }
    }

    fn message(&self) -> Option<serde_json::Value> {
        Some(serde_json::Value::String(self.to_string()))
    }
}
