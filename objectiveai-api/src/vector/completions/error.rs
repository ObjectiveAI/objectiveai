//! Error types for vector completion operations.

/// Errors that can occur during vector completion creation.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The profile weights are invalid.
    #[error("invalid profile: {0}")]
    InvalidProfile(String),
    /// Failed to fetch votes from a previous completion for retry.
    #[error("fetch retry error: {0}")]
    FetchRetry(objectiveai::error::ResponseError),
    /// The completion specified for retry was not found.
    #[error("retry not found")]
    RetryNotFound,
    /// Failed to fetch a vote from the global cache.
    #[error("fetch cache vote error: {0}")]
    FetchCacheVote(objectiveai::error::ResponseError),
    /// Failed to fetch the Ensemble definition.
    #[error("fetch ensemble error: {0}")]
    FetchEnsemble(objectiveai::error::ResponseError),
    /// The requested Ensemble was not found.
    #[error("ensemble not found")]
    EnsembleNotFound,
    /// The provided Ensemble definition is invalid.
    #[error("invalid ensemble: {0}")]
    InvalidEnsemble(String),
    /// Vector completions require at least two response options.
    #[error("expected two or more request vector responses, got {0}")]
    ExpectedTwoOrMoreRequestVectorResponses(usize),
}

impl objectiveai::error::StatusError for Error {
    fn status(&self) -> u16 {
        match self {
            Error::InvalidProfile(_) => 400,
            Error::FetchRetry(e) => e.status(),
            Error::RetryNotFound => 404,
            Error::FetchCacheVote(e) => e.status(),
            Error::FetchEnsemble(e) => e.status(),
            Error::EnsembleNotFound => 404,
            Error::InvalidEnsemble(_) => 400,
            Error::ExpectedTwoOrMoreRequestVectorResponses(_) => 400,
        }
    }

    fn message(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "kind": "vector",
            "error": match self {
                Error::InvalidProfile(msg) => serde_json::json!({
                    "kind": "invalid_profile",
                    "error": msg,
                }),
                Error::FetchRetry(e) => serde_json::json!({
                    "kind": "fetch_retry",
                    "error": e.message(),
                }),
                Error::RetryNotFound => serde_json::json!({
                    "kind": "retry_not_found",
                    "error": "retry not found",
                }),
                Error::FetchCacheVote(e) => serde_json::json!({
                    "kind": "fetch_cache_vote",
                    "error": e.message(),
                }),
                Error::FetchEnsemble(e) => serde_json::json!({
                    "kind": "fetch_ensemble",
                    "error": e.message(),
                }),
                Error::EnsembleNotFound => serde_json::json!({
                    "kind": "ensemble_not_found",
                    "error": "ensemble not found",
                }),
                Error::InvalidEnsemble(msg) => serde_json::json!({
                    "kind": "invalid_ensemble",
                    "error": msg,
                }),
                Error::ExpectedTwoOrMoreRequestVectorResponses(n) => serde_json::json!({
                    "kind": "expected_two_or_more_request_vector_responses",
                    "error": format!("expected two or more request vector responses, got {}", n),
                }),
            }
        }))
    }
}
