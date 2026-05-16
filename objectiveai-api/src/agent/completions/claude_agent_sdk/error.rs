#[derive(Debug, PartialEq, thiserror::Error)]
pub enum Error {
    #[error("rate limited")]
    RateLimit,

    #[error("invalid continuation: {0}")]
    InvalidContinuation(String),

    #[error("BYOK is not supported for Claude Agent SDK")]
    InvalidByok,

    #[error("invalid messages: {0}")]
    InvalidMessages(String),

    #[error("unsupported response format")]
    UnsupportedResponseFormat,

    #[error("spawn error: {0}")]
    Spawn(String),

    #[error("IO error: {0}")]
    Io(String),

    #[error("deserialization error: {0}")]
    Json(String),

    #[error("stderr: {0}")]
    Stderr(String),

    #[error("no output from subprocess")]
    NoOutput,

    #[error("Claude Agent SDK is not enabled")]
    NotEnabled,
}

impl objectiveai_sdk::error::StatusError for Error {
    fn status(&self) -> u16 {
        match self {
            Self::RateLimit => 429,
            Self::InvalidContinuation(_) => 400,
            Self::InvalidByok => 400,
            Self::InvalidMessages(_) => 400,
            Self::UnsupportedResponseFormat => 400,
            Self::NotEnabled => 400,
            Self::Spawn(_) => 500,
            Self::Io(_) => 500,
            Self::Json(_) => 500,
            Self::Stderr(_) => 500,
            Self::NoOutput => 500,
        }
    }

    fn message(&self) -> Option<serde_json::Value> {
        Some(serde_json::Value::String(self.to_string()))
    }
}
