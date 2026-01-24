//! Error types for Function execution.

use crate::vector;

/// Errors that can occur during Function execution.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Failed to fetch a Function definition.
    #[error("fetch function error: {0}")]
    FetchFunction(objectiveai::error::ResponseError),
    /// The requested Function was not found.
    #[error("function not found")]
    FunctionNotFound,
    /// Failed to fetch a Profile definition.
    #[error("fetch profile error: {0}")]
    FetchProfile(objectiveai::error::ResponseError),
    /// The requested Profile was not found.
    #[error("profile not found")]
    ProfileNotFound,
    /// The Profile is invalid for the Function.
    #[error("invalid profile")]
    InvalidProfile,
    /// Failed to fetch an Ensemble definition.
    #[error("fetch ensemble error: {0}")]
    FetchEnsemble(objectiveai::error::ResponseError),
    /// The requested Ensemble was not found.
    #[error("ensemble not found")]
    EnsembleNotFound,
    /// The Ensemble definition is invalid.
    #[error("invalid ensemble: {0}")]
    InvalidEnsemble(String),
    /// Failed to fetch retry data.
    #[error("fetch retry error: {0}")]
    FetchRetry(objectiveai::error::ResponseError),
    /// The retry data was not found.
    #[error("retry not found")]
    RetryNotFound,
    /// The retry token is malformed.
    #[error("invalid retry token")]
    InvalidRetryToken,
    /// A JMESPath expression in the Function is invalid.
    #[error("invalid function expression: {0}")]
    InvalidAppExpression(
        #[from] objectiveai::functions::expression::ExpressionError,
    ),
    /// A Vector Completion task failed.
    #[error("vector completion error: {0}")]
    Vector(#[from] vector::completions::Error),
    /// The input does not match the Function's input schema.
    #[error("Input does not match function input schema")]
    InputSchemaMismatch,
    /// Scalar output is not in [0, 1] range.
    #[error("invalid scalar output, expected number between 0 and 1")]
    InvalidScalarOutput,
    /// Vector output does not sum to 1 or has wrong length.
    #[error(
        "invalid vector output, expected vector of numbers summing to 1 of length {0}"
    )]
    InvalidVectorOutput(usize),
}

impl objectiveai::error::StatusError for Error {
    fn status(&self) -> u16 {
        match self {
            Error::FetchFunction(e) => e.status(),
            Error::FunctionNotFound => 404,
            Error::FetchProfile(e) => e.status(),
            Error::ProfileNotFound => 404,
            Error::InvalidProfile => 400,
            Error::FetchEnsemble(e) => e.status(),
            Error::EnsembleNotFound => 404,
            Error::InvalidEnsemble(_) => 400,
            Error::FetchRetry(e) => e.status(),
            Error::RetryNotFound => 404,
            Error::InvalidRetryToken => 400,
            Error::InvalidAppExpression(_) => 400,
            Error::Vector(e) => e.status(),
            Error::InputSchemaMismatch => 400,
            Error::InvalidScalarOutput => 400,
            Error::InvalidVectorOutput(_) => 400,
        }
    }

    fn message(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "kind": "vector",
            "error": match self {
                Error::FetchFunction(e) => serde_json::json!({
                    "kind": "fetch_function",
                    "error": e.message(),
                }),
                Error::FunctionNotFound => serde_json::json!({
                    "kind": "function_not_found",
                    "error": "function not found",
                }),
                Error::FetchProfile(e) => serde_json::json!({
                    "kind": "fetch_profile",
                    "error": e.message(),
                }),
                Error::ProfileNotFound => serde_json::json!({
                    "kind": "profile_not_found",
                    "error": "profile not found",
                }),
                Error::InvalidProfile => serde_json::json!({
                    "kind": "invalid_profile",
                    "error": "invalid profile"
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
                Error::FetchRetry(e) => serde_json::json!({
                    "kind": "fetch_retry",
                    "error": e.message(),
                }),
                Error::RetryNotFound => serde_json::json!({
                    "kind": "retry_not_found",
                    "error": "retry not found",
                }),
                Error::InvalidRetryToken => serde_json::json!({
                    "kind": "invalid_retry_token",
                    "error": "invalid retry token",
                }),
                Error::InvalidAppExpression(e) => serde_json::json!({
                    "kind": "invalid_expression",
                    "error": e.to_string(),
                }),
                Error::Vector(e) => serde_json::json!({
                    "kind": "vector_completion",
                    "error": e.message(),
                }),
                Error::InputSchemaMismatch => serde_json::json!({
                    "kind": "input_schema_mismatch",
                    "error": "Input does not match function input schema",
                }),
                Error::InvalidScalarOutput => serde_json::json!({
                    "kind": "invalid_scalar_output",
                    "error": "invalid scalar output, expected number between 0 and 1",
                }),
                Error::InvalidVectorOutput(len) => serde_json::json!({
                    "kind": "invalid_vector_output",
                    "error": format!("invalid vector output, expected vector of numbers summing to 1 of length {}", len),
                }),
            }
        }))
    }
}
