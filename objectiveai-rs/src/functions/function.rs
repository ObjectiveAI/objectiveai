//! Function types and client-side compilation.

use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

/// A Function definition, either remote (GitHub-hosted) or inline.
///
/// Functions are composable scoring pipelines that transform structured input
/// into scores. Use [`compile_tasks`](Self::compile_tasks) and
/// [`compile_output`](Self::compile_output) to preview how expressions resolve
/// for given inputs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Function {
    /// A GitHub-hosted function with metadata (description, schema, changelog, etc.).
    Remote(RemoteFunction),
    /// An inline function definition without metadata.
    Inline(InlineFunction),
}

impl Function {
    /// Compiles task expressions to show the final tasks for a given input.
    ///
    /// Evaluates all JMESPath expressions in the function's tasks using the
    /// provided input data. Tasks with `skip` expressions that evaluate to true
    /// return `None`. Tasks with `map` fields produce multiple task instances.
    ///
    /// # Returns
    ///
    /// A vector where each element corresponds to a task definition:
    /// - `None` if the task was skipped
    /// - `Some(CompiledTask::One(...))` for non-mapped tasks
    /// - `Some(CompiledTask::Many(...))` for mapped tasks
    pub fn compile_tasks(
        self,
        input: &super::expression::Input,
    ) -> Result<
        Vec<Option<super::CompiledTask>>,
        super::expression::ExpressionError,
    > {
        static EMPTY_TASKS: LazyLock<
            Vec<Option<super::expression::TaskOutput>>,
        > = LazyLock::new(|| Vec::new());

        // extract input_maps expression and task expressions
        let (input_maps_expr, task_exprs) = match self {
            Function::Remote(RemoteFunction::Scalar {
                input_maps,
                tasks,
                ..
            }) => (input_maps, tasks),
            Function::Remote(RemoteFunction::Vector {
                input_maps,
                tasks,
                ..
            }) => (input_maps, tasks),
            Function::Inline(InlineFunction::Scalar {
                input_maps,
                tasks,
                ..
            }) => (input_maps, tasks),
            Function::Inline(InlineFunction::Vector {
                input_maps,
                tasks,
                ..
            }) => (input_maps, tasks),
        };

        // prepare params for compiling expressions
        let mut params =
            super::expression::Params::Ref(super::expression::ParamsRef {
                input,
                tasks: &EMPTY_TASKS,
                map: None,
            });

        // compile input_maps
        let input_maps = if let Some(input_maps_expr) = input_maps_expr {
            Some(input_maps_expr.compile(&params)?)
        } else {
            None
        };

        // compile tasks
        let mut tasks = Vec::with_capacity(task_exprs.len());
        for mut task_expr in task_exprs {
            tasks.push(
                if let Some(skip_expr) = task_expr.take_skip()
                    && skip_expr.compile_one::<bool>(&params)?
                {
                    // None if task is skipped
                    None
                } else if let Some(input_map_index) = task_expr.input_map() {
                    // for map tasks, map input to multiple instances of the task
                    if let Some(input_maps) = &input_maps
                        && let Some(input_map) =
                            input_maps.get(input_map_index as usize)
                    {
                        // compile task for each map input
                        let mut map_tasks = Vec::with_capacity(input_map.len());
                        for input in input_map {
                            // set map input
                            match &mut params {
                                super::expression::Params::Ref(params_ref) => {
                                    params_ref.map = Some(input);
                                }
                                _ => unreachable!(),
                            }
                            // compile task with map input
                            map_tasks.push(task_expr.clone().compile(&params)?);
                            // reset map input
                            match &mut params {
                                super::expression::Params::Ref(params_ref) => {
                                    params_ref.map = None;
                                }
                                _ => unreachable!(),
                            }
                        }
                        Some(super::CompiledTask::Many(map_tasks))
                    } else {
                        // no map found is treated as empty map
                        Some(super::CompiledTask::Many(Vec::new()))
                    }
                } else {
                    // compile single task
                    Some(super::CompiledTask::One(task_expr.compile(&params)?))
                },
            );
        }

        // compiled tasks
        Ok(tasks)
    }

    /// Computes the final output given input and task outputs.
    ///
    /// Evaluates the function's output expression using the provided input data
    /// and task results. Also validates that the output meets constraints:
    /// - Scalar functions: output must be in [0, 1]
    /// - Vector functions: output must sum to approximately 1
    pub fn compile_output(
        self,
        input: &super::expression::Input,
        task_outputs: &[Option<super::expression::TaskOutput>],
    ) -> Result<
        super::expression::CompiledFunctionOutput,
        super::expression::ExpressionError,
    > {
        #[derive(Clone, Copy)]
        enum FunctionType {
            Scalar,
            Vector,
        }
        static EMPTY_TASKS: LazyLock<
            Vec<Option<super::expression::TaskOutput>>,
        > = LazyLock::new(|| Vec::new());

        // prepare params for compiling output_length expression
        let mut params =
            super::expression::Params::Ref(super::expression::ParamsRef {
                input,
                tasks: &EMPTY_TASKS,
                map: None,
            });

        // extract output expression and output_length
        let (function_type, output_expr, output_length) = match self {
            Function::Remote(RemoteFunction::Scalar { output, .. }) => {
                (FunctionType::Scalar, output, None)
            }
            Function::Remote(RemoteFunction::Vector {
                output,
                output_length,
                ..
            }) => (
                FunctionType::Vector,
                output,
                Some(output_length.compile_one(&params)?),
            ),
            Function::Inline(InlineFunction::Scalar { output, .. }) => {
                (FunctionType::Scalar, output, None)
            }
            Function::Inline(InlineFunction::Vector { output, .. }) => {
                (FunctionType::Vector, output, None)
            }
        };

        // prepare params for compiling output expression
        match &mut params {
            super::expression::Params::Ref(params_ref) => {
                params_ref.tasks = task_outputs;
            }
            _ => unreachable!(),
        }

        // compile output
        let output = output_expr
            .compile_one::<super::expression::FunctionOutput>(&params)?;

        // validate output
        let valid = match (function_type, &output, output_length) {
            (
                FunctionType::Scalar,
                &super::expression::FunctionOutput::Scalar(scalar),
                _,
            ) => {
                scalar >= rust_decimal::Decimal::ZERO
                    && scalar <= rust_decimal::Decimal::ONE
            }
            (
                FunctionType::Vector,
                super::expression::FunctionOutput::Vector(vector),
                Some(length),
            ) => {
                let sum = vector.iter().sum::<rust_decimal::Decimal>();
                vector.len() == length as usize
                    && sum >= rust_decimal::dec!(0.99)
                    && sum <= rust_decimal::dec!(1.01)
            }
            (
                FunctionType::Vector,
                super::expression::FunctionOutput::Vector(vector),
                None,
            ) => {
                let sum = vector.iter().sum::<rust_decimal::Decimal>();
                sum >= rust_decimal::dec!(0.99)
                    && sum <= rust_decimal::dec!(1.01)
            }
            _ => false,
        };

        // compiled output
        Ok(super::expression::CompiledFunctionOutput { output, valid })
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

    /// Returns the function's changelog, if available.
    pub fn changelog(&self) -> Option<&str> {
        match self {
            Function::Remote(remote_function) => remote_function.changelog(),
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

    /// Returns the function's input maps, if defined.
    pub fn input_maps(&self) -> Option<&super::expression::InputMaps> {
        match self {
            Function::Remote(remote_function) => remote_function.input_maps(),
            Function::Inline(inline_function) => inline_function.input_maps(),
        }
    }

    /// Returns the function's tasks.
    pub fn tasks(&self) -> &[super::TaskExpression] {
        match self {
            Function::Remote(remote_function) => remote_function.tasks(),
            Function::Inline(inline_function) => inline_function.tasks(),
        }
    }

    /// Returns the function's output expression.
    pub fn output(&self) -> &super::expression::Expression {
        match self {
            Function::Remote(remote_function) => remote_function.output(),
            Function::Inline(inline_function) => inline_function.output(),
        }
    }

    /// Returns the function's expected output length expression, if defined.
    pub fn output_length(
        &self,
    ) -> Option<&super::expression::WithExpression<u64>> {
        match self {
            Function::Remote(remote_function) => {
                remote_function.output_length()
            }
            Function::Inline(_) => None,
        }
    }

    /// Returns the function's input_split expression, if defined.
    pub fn input_split(
        &self,
    ) -> Option<&super::expression::WithExpression<Vec<super::expression::Input>>>
    {
        match self {
            Function::Remote(remote_function) => remote_function.input_split(),
            Function::Inline(inline_function) => inline_function.input_split(),
        }
    }

    /// Returns the function's input_merge expression, if defined.
    pub fn input_merge(
        &self,
    ) -> Option<&super::expression::WithExpression<super::expression::Input>>
    {
        match self {
            Function::Remote(remote_function) => remote_function.input_merge(),
            Function::Inline(inline_function) => inline_function.input_merge(),
        }
    }
}

/// A GitHub-hosted function with full metadata.
///
/// Remote functions are stored as `function.json` in GitHub repositories and
/// referenced by `owner/repository`. They include documentation fields that
/// inline functions lack.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RemoteFunction {
    /// Produces a single score in [0, 1].
    #[serde(rename = "scalar.function")]
    Scalar {
        /// Human-readable description of what the function does.
        description: String,
        /// Version history and changes for this function.
        #[serde(skip_serializing_if = "Option::is_none")]
        changelog: Option<String>,
        /// JSON Schema defining the expected input structure.
        input_schema: super::expression::InputSchema,
        /// Expressions that transform input into a 2D array for mapped tasks.
        /// Each sub-array can be referenced by tasks via their `map` index.
        /// Receives: `input`.
        #[serde(skip_serializing_if = "Option::is_none")]
        input_maps: Option<super::expression::InputMaps>,
        /// The list of tasks to execute. Tasks with a `map` index are expanded
        /// into multiple instances, one per element in the referenced sub-array.
        /// Each instance is compiled with `map` set to that element's value.
        /// Receives: `input`, `map` (if mapped).
        tasks: Vec<super::TaskExpression>,
        /// Expression computing the final score from task results.
        /// Receives: `input`, `tasks`.
        output: super::expression::Expression,
    },
    /// Produces a vector of scores that sums to 1.
    #[serde(rename = "vector.function")]
    Vector {
        /// Human-readable description of what the function does.
        description: String,
        /// Version history and changes for this function.
        #[serde(skip_serializing_if = "Option::is_none")]
        changelog: Option<String>,
        /// JSON Schema defining the expected input structure.
        input_schema: super::expression::InputSchema,
        /// Expressions that transform input into a 2D array for mapped tasks.
        /// Each sub-array can be referenced by tasks via their `map` index.
        /// Receives: `input`.
        #[serde(skip_serializing_if = "Option::is_none")]
        input_maps: Option<super::expression::InputMaps>,
        /// The list of tasks to execute. Tasks with a `map` index are expanded
        /// into multiple instances, one per element in the referenced sub-array.
        /// Each instance is compiled with `map` set to that element's value.
        /// Receives: `input`, `map` (if mapped).
        tasks: Vec<super::TaskExpression>,
        /// Expression computing the final score vector from task results.
        /// Receives: `input`, `tasks`.
        output: super::expression::Expression,
        /// Expression computing the expected output vector length.
        /// Receives: `input`.
        output_length: super::expression::WithExpression<u64>,
        /// Expression transforming input into an input array of the output_length
        /// When the Function is executed with any input from the array,
        /// The output_length should be 1.
        /// Receives: `input`.
        input_split:
            super::expression::WithExpression<Vec<super::expression::Input>>,
        /// Expression transforming an array of inputs computed by `input_split`
        /// into a single Input object for the Function.
        /// Receives: `input` (as an array).
        input_merge:
            super::expression::WithExpression<super::expression::Input>,
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

    /// Returns the function's changelog, if present.
    pub fn changelog(&self) -> Option<&str> {
        match self {
            RemoteFunction::Scalar { changelog, .. } => changelog.as_deref(),
            RemoteFunction::Vector { changelog, .. } => changelog.as_deref(),
        }
    }

    /// Returns the function's input schema.
    pub fn input_schema(&self) -> &super::expression::InputSchema {
        match self {
            RemoteFunction::Scalar { input_schema, .. } => input_schema,
            RemoteFunction::Vector { input_schema, .. } => input_schema,
        }
    }

    /// Returns the function's input maps, if defined.
    pub fn input_maps(&self) -> Option<&super::expression::InputMaps> {
        match self {
            RemoteFunction::Scalar { input_maps, .. } => input_maps.as_ref(),
            RemoteFunction::Vector { input_maps, .. } => input_maps.as_ref(),
        }
    }

    /// Returns the function's tasks.
    pub fn tasks(&self) -> &[super::TaskExpression] {
        match self {
            RemoteFunction::Scalar { tasks, .. } => tasks,
            RemoteFunction::Vector { tasks, .. } => tasks,
        }
    }

    /// Returns the function's output expression.
    pub fn output(&self) -> &super::expression::Expression {
        match self {
            RemoteFunction::Scalar { output, .. } => output,
            RemoteFunction::Vector { output, .. } => output,
        }
    }

    /// Returns the function's expected output length, if defined (vector functions only).
    pub fn output_length(
        &self,
    ) -> Option<&super::expression::WithExpression<u64>> {
        match self {
            RemoteFunction::Scalar { .. } => None,
            RemoteFunction::Vector { output_length, .. } => Some(output_length),
        }
    }

    /// Returns the function's input_split expression, if defined (vector functions only).
    pub fn input_split(
        &self,
    ) -> Option<&super::expression::WithExpression<Vec<super::expression::Input>>>
    {
        match self {
            RemoteFunction::Scalar { .. } => None,
            RemoteFunction::Vector { input_split, .. } => Some(input_split),
        }
    }

    /// Returns the function's input_merge expression, if defined (vector functions only).
    pub fn input_merge(
        &self,
    ) -> Option<&super::expression::WithExpression<super::expression::Input>>
    {
        match self {
            RemoteFunction::Scalar { .. } => None,
            RemoteFunction::Vector { input_merge, .. } => Some(input_merge),
        }
    }
}

/// An inline function definition without metadata.
///
/// Used when embedding function logic directly in requests rather than
/// referencing a GitHub-hosted function. Lacks description, changelog,
/// and input schema fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum InlineFunction {
    /// Produces a single score in [0, 1].
    #[serde(rename = "scalar.function")]
    Scalar {
        /// Expressions that transform input into a 2D array for mapped tasks.
        /// Each sub-array can be referenced by tasks via their `map` index.
        /// Receives: `input`.
        #[serde(skip_serializing_if = "Option::is_none")]
        input_maps: Option<super::expression::InputMaps>,
        /// The list of tasks to execute. Tasks with a `map` index are expanded
        /// into multiple instances, one per element in the referenced sub-array.
        /// Each instance is compiled with `map` set to that element's value.
        /// Receives: `input`, `map` (if mapped).
        tasks: Vec<super::TaskExpression>,
        /// Expression computing the final score from task results.
        /// Receives: `input`, `tasks`.
        output: super::expression::Expression,
    },
    /// Produces a vector of scores that sums to 1.
    #[serde(rename = "vector.function")]
    Vector {
        /// Expressions that transform input into a 2D array for mapped tasks.
        /// Each sub-array can be referenced by tasks via their `map` index.
        /// Receives: `input`.
        #[serde(skip_serializing_if = "Option::is_none")]
        input_maps: Option<super::expression::InputMaps>,
        /// The list of tasks to execute. Tasks with a `map` index are expanded
        /// into multiple instances, one per element in the referenced sub-array.
        /// Each instance is compiled with `map` set to that element's value.
        /// Receives: `input`, `map` (if mapped).
        tasks: Vec<super::TaskExpression>,
        /// Expression computing the final score vector from task results.
        /// Receives: `input`, `tasks`.
        output: super::expression::Expression,
        /// Expression transforming input into an input array of the output_length
        /// When the Function is executed with any input from the array,
        /// The output_length should be 1.
        /// Receives: `input`.
        /// Only required if the request uses a strategy that needs input splitting.
        input_split: Option<
            super::expression::WithExpression<Vec<super::expression::Input>>,
        >,
        /// Expression transforming an array of inputs computed by `input_split`
        /// into a single Input object for the Function.
        /// Receives: `input` (as an array).
        /// Only required if the request uses a strategy that needs input splitting.
        input_merge:
            Option<super::expression::WithExpression<super::expression::Input>>,
    },
}

impl InlineFunction {
    /// Returns the function's input maps, if defined.
    pub fn input_maps(&self) -> Option<&super::expression::InputMaps> {
        match self {
            InlineFunction::Scalar { input_maps, .. } => input_maps.as_ref(),
            InlineFunction::Vector { input_maps, .. } => input_maps.as_ref(),
        }
    }

    /// Returns the function's tasks.
    pub fn tasks(&self) -> &[super::TaskExpression] {
        match self {
            InlineFunction::Scalar { tasks, .. } => tasks,
            InlineFunction::Vector { tasks, .. } => tasks,
        }
    }

    /// Returns the function's output expression.
    pub fn output(&self) -> &super::expression::Expression {
        match self {
            InlineFunction::Scalar { output, .. } => output,
            InlineFunction::Vector { output, .. } => output,
        }
    }

    /// Returns the function's input_split expression, if defined (vector functions only).
    pub fn input_split(
        &self,
    ) -> Option<&super::expression::WithExpression<Vec<super::expression::Input>>>
    {
        match self {
            InlineFunction::Scalar { .. } => None,
            InlineFunction::Vector { input_split, .. } => input_split.as_ref(),
        }
    }

    /// Returns the function's input_merge expression, if defined (vector functions only).
    pub fn input_merge(
        &self,
    ) -> Option<&super::expression::WithExpression<super::expression::Input>>
    {
        match self {
            InlineFunction::Scalar { .. } => None,
            InlineFunction::Vector { input_merge, .. } => input_merge.as_ref(),
        }
    }
}
