//! Error types for Function execution.

use crate::vector;

/// Errors that can occur during Function execution.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Failed to fetch a Function definition.
    #[error("fetch function error: {0}")]
    FetchFunction(objectiveai_sdk::error::ResponseError),
    /// The requested Function was not found.
    #[error("function not found")]
    FunctionNotFound,
    /// Failed to fetch a Profile definition.
    #[error("fetch profile error: {0}")]
    FetchProfile(objectiveai_sdk::error::ResponseError),
    /// The requested Profile was not found.
    #[error("profile not found")]
    ProfileNotFound,
    /// The Profile is invalid for the Function.
    #[error("invalid profile: {0}")]
    InvalidProfile(String),
    /// Failed to fetch an Swarm definition.
    #[error("fetch swarm error: {0}")]
    FetchSwarm(objectiveai_sdk::error::ResponseError),
    /// The requested Swarm was not found.
    #[error("swarm not found")]
    SwarmNotFound,
    /// The Swarm definition is invalid.
    #[error("invalid swarm: {0}")]
    InvalidSwarm(String),
    /// An expression (JMESPath or Starlark) in the Function is invalid.
    #[error("invalid function expression: {0}")]
    InvalidAppExpression(
        #[from] objectiveai_sdk::functions::expression::ExpressionError,
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
    /// Invalid Function for Strategy
    #[error("invalid function for strategy: {0}")]
    InvalidFunctionForStrategy(String),
    /// Invalid Strategy
    #[error("invalid strategy: {0}")]
    InvalidStrategy(String),
    /// No valid task outputs to combine.
    #[error("no valid task outputs")]
    NoValidTaskOutputs,
    /// One or more task output expressions failed.
    #[error("task output expression errors: {0:?}")]
    TaskOutputExpressionErrors(Vec<TaskOutputExpressionError>),
    /// A circular dependency was detected between functions.
    #[error("circular dependency detected: {0:?}")]
    CircularDependency(objectiveai_sdk::RemotePath),
    /// Split requires input to be an array.
    #[error("split requires input to be an array")]
    SplitInputNotArray,
}

/// Error from evaluating a task's output expression.
#[derive(Debug, Clone)]
pub struct TaskOutputExpressionError {
    /// Index of the task that failed.
    pub task_index: usize,
    /// Description of the error.
    pub message: String,
}

impl objectiveai_sdk::error::StatusError for Error {
    fn status(&self) -> u16 {
        match self {
            Error::FetchFunction(e) => e.status(),
            Error::FunctionNotFound => 404,
            Error::FetchProfile(e) => e.status(),
            Error::ProfileNotFound => 404,
            Error::InvalidProfile(_) => 400,
            Error::FetchSwarm(e) => e.status(),
            Error::SwarmNotFound => 404,
            Error::InvalidSwarm(_) => 400,
            Error::InvalidAppExpression(_) => 400,
            Error::Vector(e) => e.status(),
            Error::InputSchemaMismatch => 400,
            Error::InvalidScalarOutput => 400,
            Error::InvalidVectorOutput(_) => 400,
            Error::InvalidFunctionForStrategy(_) => 400,
            Error::InvalidStrategy(_) => 400,
            Error::NoValidTaskOutputs => 400,
            Error::TaskOutputExpressionErrors(_) => 400,
            Error::CircularDependency(_) => 400,
            Error::SplitInputNotArray => 400,
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
                Error::InvalidFunctionForStrategy(msg) => serde_json::json!({
                    "kind": "invalid_function_for_strategy",
                    "error": msg,
                }),
                Error::InvalidStrategy(msg) => serde_json::json!({
                    "kind": "invalid_strategy",
                    "error": msg,
                }),
                Error::NoValidTaskOutputs => serde_json::json!({
                    "kind": "no_valid_task_outputs",
                    "error": "no valid task outputs to combine",
                }),
                Error::TaskOutputExpressionErrors(errors) => serde_json::json!({
                    "kind": "task_output_expression_errors",
                    "errors": errors.iter().map(|e| serde_json::json!({
                        "task_index": e.task_index,
                        "message": e.message,
                    })).collect::<Vec<_>>(),
                }),
                Error::CircularDependency(path) => serde_json::json!({
                    "kind": "circular_dependency",
                    "error": format!("circular dependency detected: {}", path.url()),
                }),
                Error::SplitInputNotArray => serde_json::json!({
                    "kind": "split_input_not_array",
                    "error": "split requires input to be an array",
                }),
            }
        }))
    }
}
