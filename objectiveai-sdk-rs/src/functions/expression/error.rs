//! Errors that can occur during expression compilation.

/// Errors that can occur when compiling expressions.
#[derive(Debug, thiserror::Error)]
pub enum ExpressionError {
    /// The JMESPath expression is invalid or failed to evaluate.
    #[error(transparent)]
    JmespathError(#[from] jmespath::JmespathError),
    /// The Starlark expression failed to parse.
    #[error("starlark parse error: {0}")]
    StarlarkParseError(String),
    /// The Starlark expression failed to evaluate.
    #[error("starlark evaluation error: {0}")]
    StarlarkEvalError(String),
    /// The Starlark result could not be converted to JSON.
    #[error("starlark conversion error: {0}")]
    StarlarkConversionError(String),
    /// The expression result could not be deserialized to the expected type.
    #[error(transparent)]
    DeserializationError(#[from] serde_json::Error),
    /// Expected a single value but the expression returned multiple.
    #[error("expected one value, found many")]
    ExpectedOneValueFoundMany,
    /// A Special expression variant is not supported for this target type.
    #[error("unsupported special expression")]
    UnsupportedSpecial,
}
