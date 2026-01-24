//! Error types for upstream provider operations.

/// Errors that can occur when communicating with upstream providers.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Error from the OpenRouter provider.
    #[error("openrouter error: {0}")]
    OpenRouter(#[from] super::openrouter::Error),
    /// Failed to fetch a BYOK API key.
    #[error("fetch BYOK error: {0}")]
    FetchByok(objectiveai::error::ResponseError),
    /// Multiple errors occurred during fallback attempts.
    #[error("multiple upstream errors: {0:?}")]
    MultipleErrors(Vec<Error>),
    /// The upstream returned an empty stream.
    #[error("empty upstream stream")]
    EmptyStream,
}

impl objectiveai::error::StatusError for Error {
    fn status(&self) -> u16 {
        match self {
            Error::OpenRouter(e) => e.status(),
            Error::FetchByok(e) => e.status(),
            Error::MultipleErrors(_) => 500,
            Error::EmptyStream => 500,
        }
    }

    fn message(&self) -> Option<serde_json::Value> {
        match self {
            Error::OpenRouter(e) => e.message(),
            Error::FetchByok(e) => e.message(),
            Error::MultipleErrors(errors) => Some(serde_json::json!({
                "kind": "multiple_upstream_errors",
                "errors": errors.iter().map(|e| {
                    serde_json::json!({
                        "status": e.status(),
                        "message": e.message(),
                    })
                }).collect::<Vec<_>>(),
            })),
            Error::EmptyStream => Some(serde_json::json!({
                "kind": "empty_upstream_stream",
            })),
        }
    }
}
