//! Error types for vector completion operations.

/// Errors that can occur during vector completion creation.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The profile weights are invalid.
    #[error("invalid profile: {0}")]
    InvalidProfile(String),
    /// Failed to fetch the Swarm definition.
    #[error("fetch swarm error: {0}")]
    FetchSwarm(objectiveai_sdk::error::ResponseError),
    /// The requested Swarm was not found.
    #[error("swarm not found")]
    SwarmNotFound,
    /// The provided Swarm definition is invalid.
    #[error("invalid swarm: {0}")]
    InvalidSwarm(String),
    /// Vector completions require at least two response options.
    #[error("expected two or more request vector responses, got {0}")]
    ExpectedTwoOrMoreRequestVectorResponses(usize),
}

impl objectiveai_sdk::error::StatusError for Error {
    fn status(&self) -> u16 {
        match self {
            Error::InvalidProfile(_) => 400,
            Error::FetchSwarm(e) => e.status(),
            Error::SwarmNotFound => 404,
            Error::InvalidSwarm(_) => 400,
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
                Error::FetchSwarm(e) => serde_json::json!({
                    "kind": "fetch_swarm",
                    "error": e.message(),
                }),
                Error::SwarmNotFound => serde_json::json!({
                    "kind": "swarm_not_found",
                    "error": "swarm not found",
                }),
                Error::InvalidSwarm(msg) => serde_json::json!({
                    "kind": "invalid_swarm",
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
