//! Parameters and context for expression evaluation.
//!
//! Provides the context available to JMESPath expressions during compilation,
//! including the function input, task outputs, and current map element.

use crate::vector;
use serde::{Deserialize, Serialize};

/// Context for evaluating JMESPath expressions.
///
/// Contains all data accessible within expressions: `input`, `tasks`, and `map`.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum Params<'i, 't, 'to, 'm> {
    /// Owned version (for deserialization).
    Owned(ParamsOwned),
    /// Borrowed version (for efficient evaluation).
    Ref(ParamsRef<'i, 't, 'to, 'm>),
}

impl<'de> serde::Deserialize<'de>
    for Params<'static, 'static, 'static, 'static>
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let owned = ParamsOwned::deserialize(deserializer)?;
        Ok(Params::Owned(owned))
    }
}

/// Owned version of expression parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamsOwned {
    /// The function's input data.
    pub input: super::Input,
    /// Results from executed tasks. Only populated for output expressions.
    pub tasks: Vec<Option<TaskOutputOwned>>,
    /// Current map element. Only populated for mapped task expressions.
    pub map: Option<super::Input>,
}

/// Borrowed version of expression parameters.
#[derive(Debug, Clone, Serialize)]
pub struct ParamsRef<'i, 't, 'to, 'm> {
    /// The function's input data.
    pub input: &'i super::Input,
    /// Results from executed tasks. Only populated for output expressions.
    pub tasks: &'t [Option<TaskOutput<'to>>],
    /// Current map element. Only populated for mapped task expressions.
    pub map: Option<&'m super::Input>,
}

/// Output from an executed task.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum TaskOutput<'a> {
    /// Owned version.
    Owned(TaskOutputOwned),
    /// Borrowed version.
    Ref(TaskOutputRef<'a>),
}

impl<'de> serde::Deserialize<'de> for TaskOutput<'static> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let owned = TaskOutputOwned::deserialize(deserializer)?;
        Ok(TaskOutput::Owned(owned))
    }
}

/// Owned task output variants.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TaskOutputOwned {
    /// Output from a single function task.
    Function(FunctionOutput),
    /// Outputs from a mapped function task.
    MapFunction(Vec<FunctionOutput>),
    /// Output from a single vector completion task.
    VectorCompletion(VectorCompletionOutput),
    /// Outputs from a mapped vector completion task.
    MapVectorCompletion(Vec<VectorCompletionOutput>),
}

/// Borrowed task output variants.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum TaskOutputRef<'a> {
    /// Output from a single function task.
    Function(&'a FunctionOutput),
    /// Outputs from a mapped function task.
    MapFunction(&'a [FunctionOutput]),
    /// Output from a single vector completion task.
    VectorCompletion(&'a VectorCompletionOutput),
    /// Outputs from a mapped vector completion task.
    MapVectorCompletion(&'a [VectorCompletionOutput]),
}

/// Output from a vector completion task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorCompletionOutput {
    /// Individual votes from each LLM.
    pub votes: Vec<vector::completions::response::Vote>,
    /// Final weighted scores for each response option.
    pub scores: Vec<rust_decimal::Decimal>,
    /// Total weight allocated to each response option.
    pub weights: Vec<rust_decimal::Decimal>,
}

impl VectorCompletionOutput {
    /// Creates a default output with uniform scores when no votes are cast.
    pub fn default_from_request_responses_len(
        request_responses_len: usize,
    ) -> Self {
        let weights = vec![rust_decimal::Decimal::ZERO; request_responses_len];
        let scores =
            vec![
                rust_decimal::Decimal::ONE
                    / rust_decimal::Decimal::from(request_responses_len);
                request_responses_len
            ];
        Self {
            votes: Vec::new(),
            scores,
            weights,
        }
    }
}

impl From<vector::completions::response::streaming::VectorCompletionChunk>
    for VectorCompletionOutput
{
    fn from(
        vector::completions::response::streaming::VectorCompletionChunk {
            votes,
            scores,
            weights,
            ..
        }: vector::completions::response::streaming::VectorCompletionChunk,
    ) -> Self {
        VectorCompletionOutput {
            votes,
            scores,
            weights,
        }
    }
}

impl From<vector::completions::response::unary::VectorCompletion>
    for VectorCompletionOutput
{
    fn from(
        vector::completions::response::unary::VectorCompletion {
            votes,
            scores,
            weights,
            ..
        }: vector::completions::response::unary::VectorCompletion,
    ) -> Self {
        VectorCompletionOutput {
            votes,
            scores,
            weights,
        }
    }
}

/// Output from a function (scalar or vector).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FunctionOutput {
    /// A single score in [0, 1].
    Scalar(rust_decimal::Decimal),
    /// A vector of scores that sums to 1.
    Vector(Vec<rust_decimal::Decimal>),
    /// An error occurred during execution.
    Err(serde_json::Value),
}

impl FunctionOutput {
    /// Converts the output into an error variant (wrapping the value as JSON).
    pub fn into_err(self) -> Self {
        match self {
            Self::Scalar(scalar) => {
                Self::Err(serde_json::to_value(scalar).unwrap())
            }
            Self::Vector(vector) => {
                Self::Err(serde_json::to_value(vector).unwrap())
            }
            Self::Err(err) => Self::Err(err),
        }
    }
}

/// Result of compiling a function's output expression.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledFunctionOutput {
    /// The computed output value.
    pub output: FunctionOutput,
    /// Whether the output is valid. Checks that:
    /// - The output type matches the function type (scalar vs vector)
    /// - Scalar outputs are in [0, 1]
    /// - Vector outputs sum to approximately 1
    /// - Vector outputs match `output_length` if specified
    pub valid: bool,
}
