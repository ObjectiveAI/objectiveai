//! Task types for Function definitions.
//!
//! Tasks are the building blocks of Functions. Each task either calls another
//! Function or runs a Vector Completion. Tasks can be conditionally skipped
//! or mapped over arrays of inputs.
//!
//! # Output Expressions
//!
//! Each task has an `output` expression that transforms its raw result into a
//! [`TaskOutputOwned`](super::expression::TaskOutputOwned). The expression receives
//! an `output` parameter that is one of four variants:
//!
//! - `Scalar(Decimal)` - a single score
//! - `Vector(Vec<Decimal>)` - a vector of scores
//! - `Vectors(Vec<Vec<Decimal>>)` - multiple vectors (from mapped tasks)
//! - `Err(Value)` - an error
//!
//! The expression must return a `TaskOutputOwned` valid for the parent function's type:
//! - **Scalar functions**: must return `Scalar(value)` where value is in [0, 1]
//! - **Vector functions**: must return `Vector(values)` where values sum to ~1 and match the expected length
//!
//! # Output Aggregation
//!
//! The function's final output is computed as a **weighted average** of all task outputs
//! using profile weights. If a function has only one task, that task's output becomes
//! the function's output directly (with weight 1.0).

use crate::agent;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A task definition with expressions (pre-compilation).
///
/// Task expressions contain dynamic fields (JMESPath or Starlark) that are
/// resolved against input data during compilation. Use [`compile`](Self::compile)
/// to produce a concrete [`Task`].
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[serde(tag = "type")]
#[schemars(rename = "functions.TaskExpression")]
pub enum TaskExpression {
    #[schemars(title = "ScalarFunction")]
    #[serde(rename = "scalar.function")]
    ScalarFunction(ScalarFunctionTaskExpression),
    #[schemars(title = "VectorFunction")]
    #[serde(rename = "vector.function")]
    VectorFunction(VectorFunctionTaskExpression),
    #[schemars(title = "VectorCompletion")]
    #[serde(rename = "vector.completion")]
    VectorCompletion(VectorCompletionTaskExpression),
    #[schemars(title = "PlaceholderScalarFunction")]
    #[serde(rename = "placeholder.scalar.function")]
    PlaceholderScalarFunction(PlaceholderScalarFunctionTaskExpression),
    #[schemars(title = "PlaceholderVectorFunction")]
    #[serde(rename = "placeholder.vector.function")]
    PlaceholderVectorFunction(PlaceholderVectorFunctionTaskExpression),
}

impl TaskExpression {
    pub fn url(&self) -> Option<String> {
        match self {
            TaskExpression::ScalarFunction(task) => Some(task.url()),
            TaskExpression::VectorFunction(task) => Some(task.url()),
            TaskExpression::VectorCompletion(_) => None,
            TaskExpression::PlaceholderScalarFunction(_) => None,
            TaskExpression::PlaceholderVectorFunction(_) => None,
        }
    }

    /// Takes and returns the skip expression, if present.
    pub fn take_skip(&mut self) -> Option<super::expression::Expression> {
        match self {
            TaskExpression::ScalarFunction(task) => task.skip.take(),
            TaskExpression::VectorFunction(task) => task.skip.take(),
            TaskExpression::VectorCompletion(task) => task.skip.take(),
            TaskExpression::PlaceholderScalarFunction(task) => task.skip.take(),
            TaskExpression::PlaceholderVectorFunction(task) => task.skip.take(),
        }
    }

    /// Returns the map expression, if this is a mapped task.
    pub fn map(&self) -> Option<&super::expression::Expression> {
        match self {
            TaskExpression::ScalarFunction(task) => task.map.as_ref(),
            TaskExpression::VectorFunction(task) => task.map.as_ref(),
            TaskExpression::VectorCompletion(task) => task.map.as_ref(),
            TaskExpression::PlaceholderScalarFunction(task) => {
                task.map.as_ref()
            }
            TaskExpression::PlaceholderVectorFunction(task) => {
                task.map.as_ref()
            }
        }
    }

    /// Compiles the expression into a concrete [`Task`].
    pub fn compile(
        self,
        params: &super::expression::Params,
    ) -> Result<Task, super::expression::ExpressionError> {
        match self {
            TaskExpression::ScalarFunction(task) => {
                task.compile(params).map(Task::ScalarFunction)
            }
            TaskExpression::VectorFunction(task) => {
                task.compile(params).map(Task::VectorFunction)
            }
            TaskExpression::VectorCompletion(task) => {
                task.compile(params).map(Task::VectorCompletion)
            }
            TaskExpression::PlaceholderScalarFunction(task) => {
                task.compile(params).map(Task::PlaceholderScalarFunction)
            }
            TaskExpression::PlaceholderVectorFunction(task) => {
                task.compile(params).map(Task::PlaceholderVectorFunction)
            }
        }
    }
}

/// A compiled task ready for execution.
///
/// Produced by compiling a [`TaskExpression`] against input data. All
/// expressions have been resolved to concrete values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type")]
#[schemars(rename = "functions.Task")]
pub enum Task {
    /// Calls a scalar function (produces a single score).
    #[schemars(title = "ScalarFunction")]
    #[serde(rename = "scalar.function")]
    ScalarFunction(ScalarFunctionTask),
    /// Calls a vector function (produces a vector of scores).
    #[schemars(title = "VectorFunction")]
    #[serde(rename = "vector.function")]
    VectorFunction(VectorFunctionTask),
    /// Runs a vector completion.
    #[schemars(title = "VectorCompletion")]
    #[serde(rename = "vector.completion")]
    VectorCompletion(VectorCompletionTask),
    /// Placeholder scalar function (always outputs 0.5).
    #[schemars(title = "PlaceholderScalarFunction")]
    #[serde(rename = "placeholder.scalar.function")]
    PlaceholderScalarFunction(PlaceholderScalarFunctionTask),
    /// Placeholder vector function (always outputs equalized vector).
    #[schemars(title = "PlaceholderVectorFunction")]
    #[serde(rename = "placeholder.vector.function")]
    PlaceholderVectorFunction(PlaceholderVectorFunctionTask),
}

impl Task {
    pub fn compile_output(
        &self,
        input: &super::expression::InputValue,
        raw_output: super::expression::TaskOutput,
    ) -> Result<
        super::expression::TaskOutputOwned,
        super::expression::ExpressionError,
    > {
        match self {
            Task::ScalarFunction(task) => {
                task.compile_output(input, raw_output)
            }
            Task::VectorFunction(task) => {
                task.compile_output(input, raw_output)
            }
            Task::VectorCompletion(task) => {
                task.compile_output(input, raw_output)
            }
            Task::PlaceholderScalarFunction(task) => {
                task.compile_output(input, raw_output)
            }
            Task::PlaceholderVectorFunction(task) => {
                task.compile_output(input, raw_output)
            }
        }
    }
}

/// Expression for a task that calls a scalar function (pre-compilation).
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "functions.ScalarFunctionTaskExpression")]
pub struct ScalarFunctionTaskExpression {
    #[serde(flatten)]
    #[schemars(schema_with = "crate::flatten_schema::<crate::RemotePath>")]
    pub path: crate::RemotePath,

    /// If this expression evaluates to true, skip the task. Receives: `input`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub skip: Option<super::expression::Expression>,

    /// Expression that evaluates to the number of mapped task instances.
    /// Each instance receives `map` as an integer index (0-based).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub map: Option<super::expression::Expression>,

    /// Expression for the input to pass to the function.
    /// Receives: `input`, `map` (if mapped).
    pub input: super::expression::WithExpression<
        super::expression::InputValueExpression,
    >,

    /// Expression to transform the task result into a valid function output.
    ///
    /// Receives `output` which is one of 4 variants:
    /// - `Scalar(Decimal)` - a single score
    /// - `Vector(Vec<Decimal>)` - a vector of scores
    /// - `Vectors(Vec<Vec<Decimal>>)` - multiple vectors (from mapped tasks)
    /// - `Err(Value)` - an error
    ///
    /// The expression must return a `TaskOutputOwned` that is valid for the parent function's type:
    /// - For scalar functions: must return `Scalar(value)` where value is in [0, 1]
    /// - For vector functions: must return `Vector(values)` where values sum to ~1 and match the expected length
    ///
    /// The function's final output is computed as a weighted average of all task outputs using
    /// profile weights. If a function has only one task, that task's output becomes the function's
    /// output directly.
    pub output: super::expression::Expression,
}

impl ScalarFunctionTaskExpression {
    pub fn url(&self) -> String {
        self.path.url()
    }

    /// Compiles the expression into a concrete [`ScalarFunctionTask`].
    pub fn compile(
        self,
        params: &super::expression::Params,
    ) -> Result<ScalarFunctionTask, super::expression::ExpressionError> {
        let input = self.input.compile_one(params)?.compile(params)?;
        Ok(ScalarFunctionTask {
            path: self.path,
            input,
            output: self.output,
        })
    }
}

/// A compiled scalar function task ready for execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "functions.ScalarFunctionTask")]
pub struct ScalarFunctionTask {
    #[serde(flatten)]
    #[schemars(schema_with = "crate::flatten_schema::<crate::RemotePath>")]
    pub path: crate::RemotePath,
    /// The resolved input to pass to the function.
    pub input: super::expression::InputValue,
    /// Expression to transform the task result into a valid function output.
    ///
    /// Receives `output` as the nested function's result (Scalar or Vector).
    /// Must return a `TaskOutputOwned` valid for the parent function's type (scalar or vector).
    /// See [`ScalarFunctionTaskExpression::output`] for full documentation.
    pub output: super::expression::Expression,
}

impl ScalarFunctionTask {
    pub fn url(&self) -> String {
        self.path.url()
    }

    pub fn compile_output(
        &self,
        input: &super::expression::InputValue,
        raw_output: super::expression::TaskOutput,
    ) -> Result<
        super::expression::TaskOutputOwned,
        super::expression::ExpressionError,
    > {
        let params =
            super::expression::Params::Ref(super::expression::ParamsRef {
                input,
                output: Some(raw_output),
                map: None,
            });
        let compiled_output = self.output.compile_one(&params)?;
        Ok(compiled_output)
    }
}

/// Expression for a task that calls a vector function (pre-compilation).
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "functions.VectorFunctionTaskExpression")]
pub struct VectorFunctionTaskExpression {
    #[serde(flatten)]
    #[schemars(schema_with = "crate::flatten_schema::<crate::RemotePath>")]
    pub path: crate::RemotePath,

    /// If this expression evaluates to true, skip the task. Receives: `input`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub skip: Option<super::expression::Expression>,

    /// Expression that evaluates to the number of mapped task instances.
    /// Each instance receives `map` as an integer index (0-based).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub map: Option<super::expression::Expression>,

    /// Expression for the input to pass to the function.
    /// Receives: `input`, `map` (if mapped).
    pub input: super::expression::WithExpression<
        super::expression::InputValueExpression,
    >,

    /// Expression to transform the task result into a valid function output.
    ///
    /// Receives `output` which is one of 4 variants:
    /// - `Scalar(Decimal)` - a single score
    /// - `Vector(Vec<Decimal>)` - a vector of scores
    /// - `Vectors(Vec<Vec<Decimal>>)` - multiple vectors (from mapped tasks)
    /// - `Err(Value)` - an error
    ///
    /// The expression must return a `TaskOutputOwned` that is valid for the parent function's type:
    /// - For scalar functions: must return `Scalar(value)` where value is in [0, 1]
    /// - For vector functions: must return `Vector(values)` where values sum to ~1 and match the expected length
    ///
    /// The function's final output is computed as a weighted average of all task outputs using
    /// profile weights. If a function has only one task, that task's output becomes the function's
    /// output directly.
    pub output: super::expression::Expression,
}

impl VectorFunctionTaskExpression {
    pub fn url(&self) -> String {
        self.path.url()
    }

    /// Compiles the expression into a concrete [`VectorFunctionTask`].
    pub fn compile(
        self,
        params: &super::expression::Params,
    ) -> Result<VectorFunctionTask, super::expression::ExpressionError> {
        let input = self.input.compile_one(params)?.compile(params)?;
        Ok(VectorFunctionTask {
            path: self.path,
            input,
            output: self.output,
        })
    }
}

/// A compiled vector function task ready for execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "functions.VectorFunctionTask")]
pub struct VectorFunctionTask {
    #[serde(flatten)]
    #[schemars(schema_with = "crate::flatten_schema::<crate::RemotePath>")]
    pub path: crate::RemotePath,
    /// The resolved input to pass to the function.
    pub input: super::expression::InputValue,
    /// Expression to transform the task result into a valid function output.
    ///
    /// Receives `output` as the nested function's result (Scalar or Vector).
    /// Must return a `TaskOutputOwned` valid for the parent function's type (scalar or vector).
    /// See [`VectorFunctionTaskExpression::output`] for full documentation.
    pub output: super::expression::Expression,
}

impl VectorFunctionTask {
    pub fn url(&self) -> String {
        self.path.url()
    }

    pub fn compile_output(
        &self,
        input: &super::expression::InputValue,
        raw_output: super::expression::TaskOutput,
    ) -> Result<
        super::expression::TaskOutputOwned,
        super::expression::ExpressionError,
    > {
        let params =
            super::expression::Params::Ref(super::expression::ParamsRef {
                input,
                output: Some(raw_output),
                map: None,
            });
        let compiled_output = self.output.compile_one(&params)?;
        Ok(compiled_output)
    }
}

/// Expression for a task that runs a vector completion (pre-compilation).
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "functions.VectorCompletionTaskExpression")]
pub struct VectorCompletionTaskExpression {
    /// If this expression evaluates to true, skip the task. Receives: `input`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub skip: Option<super::expression::Expression>,

    /// Expression that evaluates to the number of mapped task instances.
    /// Each instance receives `map` as an integer index (0-based).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub map: Option<super::expression::Expression>,

    /// Expression for the conversation messages (the prompt).
    /// Receives: `input`, `map` (if mapped).
    pub messages: super::expression::WithExpression<
        Vec<
            super::expression::WithExpression<
                agent::completions::message::MessageExpression,
            >,
        >,
    >,
    /// Expression for the possible responses the LLMs can vote for.
    /// Receives: `input`, `map` (if mapped).
    pub responses: super::expression::WithExpression<
        Vec<
            super::expression::WithExpression<
                agent::completions::message::RichContentExpression,
            >,
        >,
    >,

    /// Expression to transform the task result into a valid function output.
    ///
    /// Receives `output` as the task's raw result (typically `Vector(scores)`).
    ///
    /// The expression must return a `TaskOutputOwned` that is valid for the parent function's type:
    /// - For scalar functions: must return `Scalar(value)` where value is in [0, 1]
    /// - For vector functions: must return `Vector(values)` where values sum to ~1 and match the expected length
    ///
    /// The function's final output is computed as a weighted average of all task outputs using
    /// profile weights. If a function has only one task, that task's output becomes the function's
    /// output directly.
    pub output: super::expression::Expression,
}

impl VectorCompletionTaskExpression {
    /// Compiles the expression into a concrete [`VectorCompletionTask`].
    pub fn compile(
        self,
        params: &super::expression::Params,
    ) -> Result<VectorCompletionTask, super::expression::ExpressionError> {
        // compile messages
        let messages = self.messages.compile_one(params)?;
        let mut compiled_messages = Vec::with_capacity(messages.len());
        for message in messages {
            match message.compile_one_or_many(params)? {
                super::expression::OneOrMany::One(one_message) => {
                    compiled_messages.push(one_message.compile(params)?);
                }
                super::expression::OneOrMany::Many(many_messages) => {
                    for message in many_messages {
                        compiled_messages.push(message.compile(params)?);
                    }
                }
            }
        }

        // compile responses
        let responses = self.responses.compile_one(params)?;
        let mut compiled_responses = Vec::with_capacity(responses.len());
        for response in responses {
            match response.compile_one_or_many(params)? {
                super::expression::OneOrMany::One(one_response) => {
                    compiled_responses.push(one_response.compile(params)?);
                }
                super::expression::OneOrMany::Many(many_responses) => {
                    for response in many_responses {
                        compiled_responses.push(response.compile(params)?);
                    }
                }
            }
        }

        Ok(VectorCompletionTask {
            messages: compiled_messages,
            responses: compiled_responses,
            output: self.output,
        })
    }
}

/// A compiled vector completion task ready for execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "functions.VectorCompletionTask")]
pub struct VectorCompletionTask {
    /// The resolved conversation messages.
    pub messages: Vec<agent::completions::message::Message>,
    /// The resolved response options the LLMs can vote for.
    pub responses: Vec<agent::completions::message::RichContent>,
    /// Expression to transform the task result into a valid function output.
    ///
    /// Receives `output` as the task's raw result (typically `Vector(scores)`).
    /// Must return a `TaskOutputOwned` valid for the parent function's type (scalar or vector).
    /// See [`VectorCompletionTaskExpression::output`] for full documentation.
    pub output: super::expression::Expression,
}

impl VectorCompletionTask {
    pub fn compile_output(
        &self,
        input: &super::expression::InputValue,
        raw_output: super::expression::TaskOutput,
    ) -> Result<
        super::expression::TaskOutputOwned,
        super::expression::ExpressionError,
    > {
        let params =
            super::expression::Params::Ref(super::expression::ParamsRef {
                input,
                output: Some(raw_output),
                map: None,
            });
        let compiled_output = self.output.compile_one(&params)?;
        Ok(compiled_output)
    }
}

/// Expression for a placeholder scalar function task (pre-compilation).
///
/// Like [`ScalarFunctionTaskExpression`] but without owner/repository/commit.
/// Always produces a fixed output of 0.5.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "functions.PlaceholderScalarFunctionTaskExpression")]
pub struct PlaceholderScalarFunctionTaskExpression {
    /// JSON Schema defining the expected input structure.
    pub input_schema: super::expression::InputSchema,

    /// If this expression evaluates to true, skip the task. Receives: `input`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub skip: Option<super::expression::Expression>,

    /// Expression that evaluates to the number of mapped task instances.
    /// Each instance receives `map` as an integer index (0-based).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub map: Option<super::expression::Expression>,

    /// Expression for the input to pass to the placeholder function.
    /// Receives: `input`, `map` (if mapped).
    pub input: super::expression::WithExpression<
        super::expression::InputValueExpression,
    >,

    /// Expression to transform the fixed 0.5 output.
    /// Receives: `input`, `output` as `Scalar(0.5)`.
    pub output: super::expression::Expression,
}

impl PlaceholderScalarFunctionTaskExpression {
    pub fn compile(
        self,
        params: &super::expression::Params,
    ) -> Result<PlaceholderScalarFunctionTask, super::expression::ExpressionError>
    {
        let input = self.input.compile_one(params)?.compile(params)?;
        Ok(PlaceholderScalarFunctionTask {
            input_schema: self.input_schema,
            input,
            output: self.output,
        })
    }
}

/// A compiled placeholder scalar function task.
///
/// Always produces `Scalar(0.5)` before the output expression
/// is applied.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "functions.PlaceholderScalarFunctionTask")]
pub struct PlaceholderScalarFunctionTask {
    /// JSON Schema defining the expected input structure.
    pub input_schema: super::expression::InputSchema,
    /// The resolved input.
    pub input: super::expression::InputValue,
    /// Expression to transform the fixed 0.5 output.
    pub output: super::expression::Expression,
}

impl PlaceholderScalarFunctionTask {
    pub fn compile_output(
        &self,
        input: &super::expression::InputValue,
        raw_output: super::expression::TaskOutput,
    ) -> Result<
        super::expression::TaskOutputOwned,
        super::expression::ExpressionError,
    > {
        let params =
            super::expression::Params::Ref(super::expression::ParamsRef {
                input,
                output: Some(raw_output),
                map: None,
            });
        let compiled_output = self.output.compile_one(&params)?;
        Ok(compiled_output)
    }
}

/// Expression for a placeholder vector function task (pre-compilation).
///
/// Like [`VectorFunctionTaskExpression`] but without owner/repository/commit.
/// Always produces an equalized vector of length `output_length`.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "functions.PlaceholderVectorFunctionTaskExpression")]
pub struct PlaceholderVectorFunctionTaskExpression {
    /// JSON Schema defining the expected input structure.
    pub input_schema: super::expression::InputSchema,

    /// Expression computing the expected output vector length.
    /// Receives: `input`.
    pub output_length: super::expression::Expression,

    /// Expression transforming input into sub-inputs for swiss system.
    /// Receives: `input`.
    pub input_split: super::expression::Expression,

    /// Expression merging sub-inputs back into one input.
    /// Receives: `input` (as an array).
    pub input_merge: super::expression::Expression,

    /// If this expression evaluates to true, skip the task. Receives: `input`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub skip: Option<super::expression::Expression>,

    /// Expression that evaluates to the number of mapped task instances.
    /// Each instance receives `map` as an integer index (0-based).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub map: Option<super::expression::Expression>,

    /// Expression for the input to pass to the placeholder function.
    /// Receives: `input`, `map` (if mapped).
    pub input: super::expression::WithExpression<
        super::expression::InputValueExpression,
    >,

    /// Expression to transform the equalized vector output.
    /// Receives: `input`, `output` as `Vector(equalized)`.
    pub output: super::expression::Expression,
}

impl PlaceholderVectorFunctionTaskExpression {
    pub fn compile(
        self,
        params: &super::expression::Params,
    ) -> Result<PlaceholderVectorFunctionTask, super::expression::ExpressionError>
    {
        let input = self.input.compile_one(params)?.compile(params)?;
        Ok(PlaceholderVectorFunctionTask {
            input_schema: self.input_schema,
            output_length: self.output_length,
            input_split: self.input_split,
            input_merge: self.input_merge,
            input,
            output: self.output,
        })
    }
}

/// A compiled placeholder vector function task.
///
/// Always produces `Vector(vec![1/N; output_length])` before
/// the output expression is applied.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "functions.PlaceholderVectorFunctionTask")]
pub struct PlaceholderVectorFunctionTask {
    /// JSON Schema defining the expected input structure.
    pub input_schema: super::expression::InputSchema,
    /// Expression computing the expected output vector length.
    pub output_length: super::expression::Expression,
    /// Expression transforming input into sub-inputs for swiss system.
    pub input_split: super::expression::Expression,
    /// Expression merging sub-inputs back into one input.
    pub input_merge: super::expression::Expression,
    /// The resolved input.
    pub input: super::expression::InputValue,
    /// Expression to transform the equalized vector output.
    pub output: super::expression::Expression,
}

impl PlaceholderVectorFunctionTask {
    pub fn compile_output(
        &self,
        input: &super::expression::InputValue,
        raw_output: super::expression::TaskOutput,
    ) -> Result<
        super::expression::TaskOutputOwned,
        super::expression::ExpressionError,
    > {
        let params =
            super::expression::Params::Ref(super::expression::ParamsRef {
                input,
                output: Some(raw_output),
                map: None,
            });
        let compiled_output = self.output.compile_one(&params)?;
        Ok(compiled_output)
    }
}

/// The result of compiling a task expression.
///
/// Tasks without a `map` field compile to a single task. Tasks with a `map`
/// expression are expanded into multiple tasks, one per integer index from
/// 0 to the evaluated count.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "functions.CompiledTask")]
pub enum CompiledTask {
    /// A single task (no mapping).
    #[schemars(title = "One")]
    One(Task),
    /// Multiple task instances from mapped execution.
    #[schemars(title = "Many")]
    Many(Vec<Task>),
}
