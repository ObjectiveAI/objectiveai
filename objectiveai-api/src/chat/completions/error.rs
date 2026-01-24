//! Error types for chat completions.

/// Errors that can occur during chat completion creation.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Error from the upstream provider.
    #[error("upstream error: {0}")]
    UpstreamError(#[from] super::upstream::Error),
    /// No upstream providers were available for the request.
    #[error("no upstreams found for request")]
    NoUpstreamsFound,
    /// Failed to fetch an Ensemble LLM definition.
    #[error("fetch Ensemble LLM error: {0}")]
    FetchEnsembleLlm(objectiveai::error::ResponseError),
    /// The requested Ensemble LLM was not found.
    #[error("Ensemble LLM not found")]
    EnsembleLlmNotFound,
    /// The Ensemble LLM definition is invalid.
    #[error("invalid Ensemble LLM: {0}")]
    InvalidEnsembleLlm(String),
    /// Multiple errors occurred during fallback attempts.
    #[error("multiple errors: {0:?}")]
    MultipleErrors(Vec<Error>),
}

impl objectiveai::error::StatusError for Error {
    fn status(&self) -> u16 {
        match self {
            Error::UpstreamError(e) => e.status(),
            Error::NoUpstreamsFound => 400,
            Error::FetchEnsembleLlm(e) => e.status(),
            Error::EnsembleLlmNotFound => 404,
            Error::InvalidEnsembleLlm(_) => 400,
            Error::MultipleErrors(_) => 500,
        }
    }

    fn message(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "kind": "chat_completion",
            "error": match self {
                Error::UpstreamError(e) => serde_json::json!({
                    "kind": "upstream_error",
                    "error": e.message(),
                }),
                Error::NoUpstreamsFound => serde_json::json!({
                    "kind": "no_upstreams_found",
                    "error": "no upstreams available for the given request",
                }),
                Error::FetchEnsembleLlm(e) => serde_json::json!({
                    "kind": "fetch_ensemble_llm",
                    "error": e.message(),
                }),
                Error::EnsembleLlmNotFound => serde_json::json!({
                    "kind": "ensemble_llm_not_found",
                    "error": "Ensemble LLM not found",
                }),
                Error::InvalidEnsembleLlm(msg) => serde_json::json!({
                    "kind": "invalid_ensemble_llm",
                    "error": msg,
                }),
                Error::MultipleErrors(errors) => serde_json::json!({
                    "kind": "multiple_errors",
                    "errors": errors.iter().map(|e| {
                        serde_json::json!({
                            "status": e.status(),
                            "message": e.message(),
                        })
                    }).collect::<Vec<_>>(),
                }),
            }
        }))
    }
}
