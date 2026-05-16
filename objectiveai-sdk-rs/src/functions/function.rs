//! Function types and client-side compilation.
//!
//! # Output Computation
//!
//! Functions do **not** have a top-level output expression. Instead, each task has its
//! own `output` expression that transforms its raw result into a [`TaskOutputOwned`].
//! The function's final output is computed as a **weighted average** of all task outputs
//! using profile weights.
//!
//! - If a function has only 1 task, that task's output becomes the function's output directly
//! - If a function has multiple tasks, each task's output is weighted and averaged
//!
//! Each task's `output` expression must return a valid `TaskOutputOwned` for the function's type:
//! - **Scalar functions**: each task must return `Scalar(value)` where value is in [0, 1]
//! - **Vector functions**: each task must return `Vector(values)` where values sum to ~1
//!
//! [`TaskOutputOwned`]: super::expression::TaskOutputOwned

use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// A Function definition, either remote or inline.
///
/// Functions are composable scoring pipelines that transform structured input
/// into scores. Each task has an `output` expression that transforms its raw result
/// into a `TaskOutputOwned`. The function's final output is the weighted average of
/// all task outputs using profile weights.
///
/// Use [`compile_tasks`](Self::compile_tasks) to preview how task expressions resolve
/// for given inputs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "functions.Function")]
pub enum Function {
    /// A remote function with metadata (description, schema, etc.).
    #[schemars(title = "Remote")]
    Remote(RemoteFunction),
    /// An inline function definition without metadata.
    #[schemars(title = "Inline")]
    Inline(InlineFunction),
}

impl Function {
    /// Validates the input against the function's input schema.
    ///
    /// For remote functions, checks whether the provided input conforms to
    /// the function's JSON Schema definition. For inline functions, returns
    /// `None` since they lack schema definitions.
    ///
    /// # Returns
    ///
    /// - `Some(true)` if the input is valid against the schema
    /// - `Some(false)` if the input is invalid
    /// - `None` for inline functions (no schema to validate against)
    pub fn validate_input(
        &self,
        input: &super::expression::InputValue,
    ) -> Option<bool> {
        match self {
            Function::Remote(remote_function) => {
                Some(remote_function.input_schema().validate_input(input))
            }
            Function::Inline(_) => None,
        }
    }

    /// Compiles task expressions to show the final tasks for a given input.
    ///
    /// Evaluates all expressions (JMESPath or Starlark) in the function's tasks
    /// using the provided input data. Tasks with `skip` expressions that evaluate
    /// to true return `None`. Tasks with `map` fields produce multiple task instances.
    ///
    /// # Returns
    ///
    /// A vector where each element corresponds to a task definition:
    /// - `None` if the task was skipped
    /// - `Some(CompiledTask::One(...))` for non-mapped tasks
    /// - `Some(CompiledTask::Many(...))` for mapped tasks
    pub fn compile_tasks(
        self,
        input: &super::expression::InputValue,
    ) -> Result<
        Vec<Option<super::CompiledTask>>,
        super::expression::ExpressionError,
    > {
        // extract task expressions
        let task_exprs = match self {
            Function::Remote(RemoteFunction::Scalar { tasks, .. }) => tasks,
            Function::Remote(RemoteFunction::Vector { tasks, .. }) => tasks,
            Function::Inline(InlineFunction::Scalar { tasks, .. }) => tasks,
            Function::Inline(InlineFunction::Vector { tasks, .. }) => tasks,
        };

        // prepare params for compiling expressions
        let mut params =
            super::expression::Params::Ref(super::expression::ParamsRef {
                input,
                output: None,
                map: None,
                tasks_min: None,
                tasks_max: None,
                depth: None,
                name: None,
                spec: None,
            });

        // compile tasks
        let mut tasks = Vec::with_capacity(task_exprs.len());
        for mut task_expr in task_exprs {
            tasks.push(
                if let Some(skip_expr) = task_expr.take_skip()
                    && skip_expr.compile_one::<bool>(&params)?
                {
                    // None if task is skipped
                    None
                } else if let Some(map_expr) = task_expr.map() {
                    // evaluate map expression to get count
                    let count: u64 = map_expr.compile_one(&params)?;
                    // compile task for each map index
                    let mut map_tasks = Vec::with_capacity(count as usize);
                    for i in 0..count {
                        // set map index
                        match &mut params {
                            super::expression::Params::Ref(params_ref) => {
                                params_ref.map = Some(i);
                            }
                            _ => unreachable!(),
                        }
                        // compile task with map index
                        map_tasks.push(task_expr.clone().compile(&params)?);
                        // reset map index
                        match &mut params {
                            super::expression::Params::Ref(params_ref) => {
                                params_ref.map = None;
                            }
                            _ => unreachable!(),
                        }
                    }
                    Some(super::CompiledTask::Many(map_tasks))
                } else {
                    // compile single task
                    Some(super::CompiledTask::One(task_expr.compile(&params)?))
                },
            );
        }

        // compiled tasks
        Ok(tasks)
    }

    // /// Computes the final output given input and task outputs.
    // ///
    // /// Evaluates the function's output expression using the provided input data
    // /// and task results. Also validates that the output meets constraints:
    // /// - Scalar functions: output must be in [0, 1]
    // /// - Vector functions: output must sum to approximately 1
    // pub fn compile_output(
    //     self,
    //     input: &super::expression::InputValue,
    //     task_outputs: &[Option<super::expression::TaskOutput>],
    // ) -> Result<
    //     super::expression::CompiledFunctionOutput,
    //     super::expression::ExpressionError,
    // > {
    //     #[derive(Clone, Copy)]
    //     enum FunctionType {
    //         Scalar,
    //         Vector,
    //     }

    //     // prepare params for compiling output_length expression
    //     let mut params =
    //         super::expression::Params::Ref(super::expression::ParamsRef {
    //             input,
    //             output: None,
    //             map: None,
    //         });

    //     // extract output expression and output_length
    //     let (function_type, output_expr, output_length) = match self {
    //         Function::Remote(RemoteFunction::Scalar { output, .. }) => {
    //             (FunctionType::Scalar, output, None)
    //         }
    //         Function::Remote(RemoteFunction::Vector {
    //             output,
    //             output_length,
    //             ..
    //         }) => (
    //             FunctionType::Vector,
    //             output,
    //             Some(output_length.compile_one(&params)?),
    //         ),
    //         Function::Inline(InlineFunction::Scalar { output, .. }) => {
    //             (FunctionType::Scalar, output, None)
    //         }
    //         Function::Inline(InlineFunction::Vector { output, .. }) => {
    //             (FunctionType::Vector, output, None)
    //         }
    //     };

    //     // prepare params for compiling output expression
    //     match &mut params {
    //         super::expression::Params::Ref(params_ref) => {
    //             params_ref.tasks = task_outputs;
    //         }
    //         _ => unreachable!(),
    //     }

    //     // compile output
    //     let output = output_expr
    //         .compile_one::<super::expression::FunctionOutput>(&params)?;

    //     // validate output
    //     let valid = match (function_type, &output, output_length) {
    //         (
    //             FunctionType::Scalar,
    //             &super::expression::FunctionOutput::Scalar(scalar),
    //             _,
    //         ) => {
    //             scalar >= rust_decimal::Decimal::ZERO
    //                 && scalar <= rust_decimal::Decimal::ONE
    //         }
    //         (
    //             FunctionType::Vector,
    //             super::expression::FunctionOutput::Vector(vector),
    //             Some(length),
    //         ) => {
    //             let sum = vector.iter().sum::<rust_decimal::Decimal>();
    //             vector.len() == length as usize
    //                 && sum >= rust_decimal::dec!(0.99)
    //                 && sum <= rust_decimal::dec!(1.01)
    //         }
    //         (
    //             FunctionType::Vector,
    //             super::expression::FunctionOutput::Vector(vector),
    //             None,
    //         ) => {
    //             let sum = vector.iter().sum::<rust_decimal::Decimal>();
    //             sum >= rust_decimal::dec!(0.99)
    //                 && sum <= rust_decimal::dec!(1.01)
    //         }
    //         _ => false,
    //     };

    //     // compiled output
    //     Ok(super::expression::CompiledFunctionOutput { output, valid })
    // }

    /// Computes the expected output length for a vector function.
    ///
    /// Evaluates the `output_length` expression to determine how many elements
    /// the output vector should contain. This is only applicable to remote
    /// vector functions which have an `output_length` field.
    ///
    /// # Arguments
    ///
    /// * `input` - The function input used to compute the output length
    ///
    /// # Returns
    ///
    /// - `Ok(Some(u64))` - The expected output length for remote vector functions
    /// - `Ok(None)` - For scalar functions or inline functions
    /// - `Err(ExpressionError)` - If the expression fails to compile
    pub fn compile_output_length(
        self,
        input: &super::expression::InputValue,
    ) -> Result<Option<u64>, super::expression::ExpressionError> {
        let output_length_expr = match self {
            Function::Remote(RemoteFunction::Scalar { .. }) => None,
            Function::Remote(RemoteFunction::Vector {
                output_length, ..
            }) => Some(output_length),
            Function::Inline(InlineFunction::Scalar { .. }) => None,
            Function::Inline(InlineFunction::Vector { .. }) => None,
        };
        match output_length_expr {
            Some(output_length_expr) => {
                // prepare params for compiling output_length expression
                let params = super::expression::Params::Ref(
                    super::expression::ParamsRef {
                        input,
                        output: None,
                        map: None,
                        tasks_min: None,
                        tasks_max: None,
                        depth: None,
                        name: None,
                        spec: None,
                    },
                );
                // compile output_length
                let output_length = output_length_expr.compile_one(&params)?;
                Ok(Some(output_length))
            }
            None => Ok(None),
        }
    }

    /// Compiles the `input_split` expression to split input into multiple sub-inputs.
    ///
    /// Used by strategies like Swiss System that need to partition input into
    /// smaller pools. The expression transforms the original input into an array
    /// of inputs, where each element can be processed independently.
    ///
    /// # Arguments
    ///
    /// * `input` - The original function input to split
    ///
    /// # Returns
    ///
    /// - `Ok(Some(Vec<Input>))` - The split inputs for vector functions with `input_split` defined
    /// - `Ok(None)` - For scalar functions or functions without `input_split`
    /// - `Err(ExpressionError)` - If the expression fails to compile
    pub fn compile_input_split(
        self,
        input: &super::expression::InputValue,
    ) -> Result<
        Option<Vec<super::expression::InputValue>>,
        super::expression::ExpressionError,
    > {
        let input_split_expr = match self {
            Function::Remote(RemoteFunction::Scalar { .. }) => None,
            Function::Remote(RemoteFunction::Vector {
                input_split, ..
            }) => Some(input_split),
            Function::Inline(InlineFunction::Scalar { .. }) => None,
            Function::Inline(InlineFunction::Vector {
                input_split, ..
            }) => input_split,
        };
        match input_split_expr {
            Some(input_split_expr) => {
                // prepare params for compiling input_split expression
                let params = super::expression::Params::Ref(
                    super::expression::ParamsRef {
                        input,
                        output: None,
                        map: None,
                        tasks_min: None,
                        tasks_max: None,
                        depth: None,
                        name: None,
                        spec: None,
                    },
                );
                // compile input_split
                let input_split = input_split_expr.compile_one(&params)?;
                Ok(Some(input_split))
            }
            None => Ok(None),
        }
    }

    /// Compiles the `input_merge` expression to merge multiple sub-inputs back into one.
    ///
    /// Used by strategies like Swiss System to recombine a subset of split inputs
    /// into a single input for pool execution. The expression transforms an array
    /// of inputs (a subset from `input_split`) into a single merged input.
    ///
    /// # Arguments
    ///
    /// * `input` - An array of inputs to merge (typically a subset from `compile_input_split`)
    ///
    /// # Returns
    ///
    /// - `Ok(Some(Input))` - The merged input for vector functions with `input_merge` defined
    /// - `Ok(None)` - For scalar functions or functions without `input_merge`
    /// - `Err(ExpressionError)` - If the expression fails to compile
    pub fn compile_input_merge(
        self,
        input: &super::expression::InputValue,
    ) -> Result<
        Option<super::expression::InputValue>,
        super::expression::ExpressionError,
    > {
        let input_merge_expr = match self {
            Function::Remote(RemoteFunction::Scalar { .. }) => None,
            Function::Remote(RemoteFunction::Vector {
                input_merge, ..
            }) => Some(input_merge),
            Function::Inline(InlineFunction::Scalar { .. }) => None,
            Function::Inline(InlineFunction::Vector {
                input_merge, ..
            }) => input_merge,
        };
        match input_merge_expr {
            Some(input_merge_expr) => {
                // prepare params for compiling input_merge expression
                let params = super::expression::Params::Ref(
                    super::expression::ParamsRef {
                        input,
                        output: None,
                        map: None,
                        tasks_min: None,
                        tasks_max: None,
                        depth: None,
                        name: None,
                        spec: None,
                    },
                );
                // compile input_merge
                let input_merge = input_merge_expr.compile_one(&params)?;
                Ok(Some(input_merge))
            }
            None => Ok(None),
        }
    }

    /// Returns the function's description, if available.
    pub fn description(&self) -> Option<&str> {
        match self {
            Function::Remote(remote_function) => {
                Some(remote_function.description())
            }
            Function::Inline(_) => None,
        }
    }

    /// Returns the function's input schema, if available.
    pub fn input_schema(&self) -> Option<&super::expression::InputSchema> {
        match self {
            Function::Remote(remote_function) => {
                Some(remote_function.input_schema())
            }
            Function::Inline(_) => None,
        }
    }

    /// Returns the function's tasks.
    pub fn tasks(&self) -> &[super::TaskExpression] {
        match self {
            Function::Remote(remote_function) => remote_function.tasks(),
            Function::Inline(inline_function) => inline_function.tasks(),
        }
    }

    /// Returns the function's expected output length expression, if defined.
    pub fn output_length(&self) -> Option<&super::expression::Expression> {
        match self {
            Function::Remote(remote_function) => {
                remote_function.output_length()
            }
            Function::Inline(_) => None,
        }
    }

    /// Returns the function's input_split expression, if defined.
    pub fn input_split(&self) -> Option<&super::expression::Expression> {
        match self {
            Function::Remote(remote_function) => remote_function.input_split(),
            Function::Inline(inline_function) => inline_function.input_split(),
        }
    }

    /// Returns the function's input_merge expression, if defined.
    pub fn input_merge(&self) -> Option<&super::expression::Expression> {
        match self {
            Function::Remote(remote_function) => remote_function.input_merge(),
            Function::Inline(inline_function) => inline_function.input_merge(),
        }
    }
}

/// A remote function with full metadata.
///
/// Remote functions are stored as `function.json` in repositories and
/// referenced by `remote/owner/repository`. They include documentation fields
/// that inline functions lack.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, arbitrary::Arbitrary)]
#[serde(tag = "type")]
#[schemars(rename = "functions.RemoteFunction")]
pub enum RemoteFunction {
    /// Produces a single score in [0, 1].
    #[schemars(title = "Scalar")]
    #[serde(rename = "scalar.function")]
    Scalar {
        /// Human-readable description of what the function does.
        description: String,
        /// JSON Schema defining the expected input structure.
        input_schema: super::expression::InputSchema,
        /// The list of tasks to execute. Tasks with a `map` expression are
        /// expanded into multiple instances. Each instance is compiled with
        /// `map` set to the current integer index.
        /// Receives: `input`, `map` (if mapped).
        tasks: Vec<super::TaskExpression>,
    },
    /// Produces a vector of scores that sums to 1.
    #[schemars(title = "Vector")]
    #[serde(rename = "vector.function")]
    Vector {
        /// Human-readable description of what the function does.
        description: String,
        /// JSON Schema defining the expected input structure.
        input_schema: super::expression::InputSchema,
        /// The list of tasks to execute. Tasks with a `map` expression are
        /// expanded into multiple instances. Each instance is compiled with
        /// `map` set to the current integer index.
        /// Receives: `input`, `map` (if mapped).
        tasks: Vec<super::TaskExpression>,
        /// Expression computing the expected output vector length for task outputs.
        /// Receives: `input`.
        output_length: super::expression::Expression,
        /// Expression transforming input into an input array of the output_length
        /// When the Function is executed with any input from the array,
        /// The output_length should be 1.
        /// Receives: `input`.
        input_split: super::expression::Expression,
        /// Expression transforming an array of inputs computed by `input_split`
        /// into a single Input object for the Function.
        /// Receives: `input` (as an array).
        input_merge: super::expression::Expression,
    },
}

impl RemoteFunction {
    /// Returns the function's description.
    pub fn description(&self) -> &str {
        match self {
            RemoteFunction::Scalar { description, .. } => description,
            RemoteFunction::Vector { description, .. } => description,
        }
    }

    /// Returns the function's input schema.
    pub fn input_schema(&self) -> &super::expression::InputSchema {
        match self {
            RemoteFunction::Scalar { input_schema, .. } => input_schema,
            RemoteFunction::Vector { input_schema, .. } => input_schema,
        }
    }

    /// Returns the function's tasks.
    pub fn tasks(&self) -> &[super::TaskExpression] {
        match self {
            RemoteFunction::Scalar { tasks, .. } => tasks,
            RemoteFunction::Vector { tasks, .. } => tasks,
        }
    }

    /// Returns the function's expected output length, if defined (vector functions only).
    pub fn output_length(&self) -> Option<&super::expression::Expression> {
        match self {
            RemoteFunction::Scalar { .. } => None,
            RemoteFunction::Vector { output_length, .. } => Some(output_length),
        }
    }

    /// Returns the function's input_split expression, if defined (vector functions only).
    pub fn input_split(&self) -> Option<&super::expression::Expression> {
        match self {
            RemoteFunction::Scalar { .. } => None,
            RemoteFunction::Vector { input_split, .. } => Some(input_split),
        }
    }

    /// Returns the function's input_merge expression, if defined (vector functions only).
    pub fn input_merge(&self) -> Option<&super::expression::Expression> {
        match self {
            RemoteFunction::Scalar { .. } => None,
            RemoteFunction::Vector { input_merge, .. } => Some(input_merge),
        }
    }

    pub fn remotes(&self) -> impl Iterator<Item = &crate::RemotePath> {
        self.tasks().iter().filter_map(|task| match task {
            super::TaskExpression::ScalarFunction(t) => Some(&t.path),
            super::TaskExpression::VectorFunction(t) => Some(&t.path),
            _ => None,
        })
    }
}

/// An inline function definition without metadata.
///
/// Used when embedding function logic directly in requests rather than
/// referencing a remote function. Lacks description and input
/// schema fields.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type")]
#[schemars(rename = "functions.InlineFunction")]
pub enum InlineFunction {
    /// Produces a single score in [0, 1].
    #[schemars(title = "Scalar")]
    #[serde(rename = "scalar.function")]
    Scalar {
        /// The list of tasks to execute. Tasks with a `map` expression are
        /// expanded into multiple instances. Each instance is compiled with
        /// `map` set to the current integer index.
        /// Receives: `input`, `map` (if mapped).
        tasks: Vec<super::TaskExpression>,
    },
    /// Produces a vector of scores that sums to 1.
    #[schemars(title = "Vector")]
    #[serde(rename = "vector.function")]
    Vector {
        /// The list of tasks to execute. Tasks with a `map` expression are
        /// expanded into multiple instances. Each instance is compiled with
        /// `map` set to the current integer index.
        /// Receives: `input`, `map` (if mapped).
        tasks: Vec<super::TaskExpression>,
        /// Expression transforming input into an input array of the output_length
        /// When the Function is executed with any input from the array,
        /// The output_length should be 1.
        /// Receives: `input`.
        /// Only required if the request uses a strategy that needs input splitting.
        input_split: Option<super::expression::Expression>,
        /// Expression transforming an array of inputs computed by `input_split`
        /// into a single Input object for the Function.
        /// Receives: `input` (as an array).
        /// Only required if the request uses a strategy that needs input splitting.
        input_merge: Option<super::expression::Expression>,
    },
}

impl InlineFunction {
    /// Returns the function's tasks.
    pub fn tasks(&self) -> &[super::TaskExpression] {
        match self {
            InlineFunction::Scalar { tasks, .. } => tasks,
            InlineFunction::Vector { tasks, .. } => tasks,
        }
    }

    /// Returns the function's input_split expression, if defined (vector functions only).
    pub fn input_split(&self) -> Option<&super::expression::Expression> {
        match self {
            InlineFunction::Scalar { .. } => None,
            InlineFunction::Vector { input_split, .. } => input_split.as_ref(),
        }
    }

    /// Returns the function's input_merge expression, if defined (vector functions only).
    pub fn input_merge(&self) -> Option<&super::expression::Expression> {
        match self {
            InlineFunction::Scalar { .. } => None,
            InlineFunction::Vector { input_merge, .. } => input_merge.as_ref(),
        }
    }

    pub fn remotes(&self) -> impl Iterator<Item = &crate::RemotePath> {
        self.tasks().iter().filter_map(|task| match task {
            super::TaskExpression::ScalarFunction(t) => Some(&t.path),
            super::TaskExpression::VectorFunction(t) => Some(&t.path),
            _ => None,
        })
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[schemars(rename = "functions.FunctionType")]
pub enum FunctionType {
    #[schemars(title = "Scalar")]
    #[serde(rename = "scalar.function")]
    Scalar,
    #[schemars(title = "Vector")]
    #[serde(rename = "vector.function")]
    Vector,
}
