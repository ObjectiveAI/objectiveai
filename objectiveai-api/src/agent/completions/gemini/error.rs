use thiserror::Error;

/// Errors raised by the Gemini SDK upstream client. Mirrors the codex
/// SDK client's error family — orchestration errors raised on the Rust
/// side while spawning, reading, or validating against the runner
/// subprocess, plus the runner-reported run/parse errors.
#[derive(Debug, Error)]
pub enum Error {
    /// A JSONL line could not be parsed into a [`super::GeminiEvent`].
    #[error("failed to parse gemini event line: {0}")]
    EventParse(#[from] serde_json::Error),

    /// The runner emitted an `end` line with `status: "error"`; the
    /// inner string is the `error` payload.
    #[error("gemini run error: {0}")]
    Run(String),

    // --- orchestration errors (Rust client side) -----------------------------
    #[error("rate limited")]
    RateLimit,

    #[error("invalid continuation: {0}")]
    InvalidContinuation(String),

    #[error("BYOK is not supported for Gemini")]
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

    #[error("Gemini is not enabled")]
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
            | Self::NotEnabled => 400,
            Self::EventParse(_)
            | Self::Run(_)
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
