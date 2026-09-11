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

    #[error("runner: {0}")]
    Runner(String),

    #[error(
        "claude code login unusable on the daemon host — run `claude` \
         there and sign in ({0})"
    )]
    Auth(String),

    #[error("no output from subprocess")]
    NoOutput,

    #[error("Claude Agent SDK is not enabled")]
    NotEnabled,
}

impl Error {
    /// Classify a structured runner failure — the NDJSON `end` frame's
    /// error text, NOT the subprocess's stderr stream (that stays
    /// [`Error::Stderr`], constructed where the stderr reader's `Fatal`
    /// lines land). An auth lapse gets its own variant and status so a
    /// missing login stops presenting as a generic 500 whose text is
    /// the word "success".
    pub(crate) fn runner(message: String) -> Self {
        let lower = message.to_ascii_lowercase();
        if lower.contains("login")
            || lower.contains("logged out")
            || lower.contains("api key")
            || lower.contains("authentication")
            || lower.contains("oauth token")
        {
            Self::Auth(message)
        } else {
            Self::Runner(message)
        }
    }
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
            Self::Runner(_) => 500,
            Self::Auth(_) => 401,
            Self::NoOutput => 500,
        }
    }

    fn message(&self) -> Option<serde_json::Value> {
        Some(serde_json::Value::String(self.to_string()))
    }
}
