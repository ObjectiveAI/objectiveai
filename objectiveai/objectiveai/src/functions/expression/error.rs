//! Errors that can occur during expression compilation.

/// Errors that can occur when compiling JMESPath expressions.
#[derive(Debug, thiserror::Error)]
pub enum ExpressionError {
    /// The JMESPath expression is invalid or failed to evaluate.
    #[error(transparent)]
    JmespathError(#[from] jmespath::JmespathError),
    /// The expression result could not be deserialized to the expected type.
    #[error(transparent)]
    DeserializationError(#[from] serde_json::Error),
    /// Expected a single value but the expression returned multiple.
    #[error("expected one value, found many")]
    ExpectedOneValueFoundMany,
}
