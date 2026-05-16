#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("git error: {0}")]
    Git(#[from] git2::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid UTF-8: {0}")]
    InvalidUtf8(#[from] std::str::Utf8Error),
    #[error("deserialization error: {0}")]
    DeserializationError(#[from] serde_path_to_error::Error<serde_json::Error>),
}

impl objectiveai_sdk::error::StatusError for Error {
    fn status(&self) -> u16 {
        match self {
            Error::Git(_) => 500,
            Error::Io(_) => 500,
            Error::InvalidUtf8(_) => 500,
            Error::DeserializationError(_) => 400,
        }
    }

    fn message(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "kind": "filesystem",
            "error": match self {
                Error::Git(e) => serde_json::json!({
                    "kind": "git",
                    "error": e.to_string(),
                }),
                Error::Io(e) => serde_json::json!({
                    "kind": "io",
                    "error": e.to_string(),
                }),
                Error::InvalidUtf8(e) => serde_json::json!({
                    "kind": "invalid_utf8",
                    "error": e.to_string(),
                }),
                Error::DeserializationError(e) => serde_json::json!({
                    "kind": "deserialization",
                    "error": e.to_string(),
                }),
            }
        }))
    }
}
