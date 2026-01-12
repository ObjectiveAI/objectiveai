use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Function {
    Remote(RemoteFunction),
    Inline(InlineFunction),
}

impl Function {
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RemoteFunction {
    #[serde(rename = "scalar.function")]
    Scalar {
        description: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        changelog: Option<String>,
        input_schema: super::expression::InputSchema,
        #[serde(skip_serializing_if = "Option::is_none")]
        input_maps: Option<super::expression::InputMaps>, // receives input
        tasks: Vec<super::TaskExpression>, // receives input + maps
        output: super::expression::Expression, // receives input + tasks
    },
    #[serde(rename = "vector.function")]
    Vector {
        description: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        changelog: Option<String>,
        input_schema: super::expression::InputSchema,
        #[serde(skip_serializing_if = "Option::is_none")]
        input_maps: Option<super::expression::InputMaps>, // receives input
        tasks: Vec<super::TaskExpression>, // receives input + maps
        output: super::expression::Expression, // receives input + tasks
        output_length: super::expression::WithExpression<u64>, // receives input
    },
}

impl RemoteFunction {
    pub fn description(&self) -> &str {
        match self {
            RemoteFunction::Scalar { description, .. } => description,
            RemoteFunction::Vector { description, .. } => description,
        }
    }

    pub fn changelog(&self) -> Option<&str> {
        match self {
            RemoteFunction::Scalar { changelog, .. } => changelog.as_deref(),
            RemoteFunction::Vector { changelog, .. } => changelog.as_deref(),
        }
    }

    pub fn input_schema(&self) -> &super::expression::InputSchema {
        match self {
            RemoteFunction::Scalar { input_schema, .. } => input_schema,
            RemoteFunction::Vector { input_schema, .. } => input_schema,
        }
    }

    pub fn input_maps(&self) -> Option<&super::expression::InputMaps> {
        match self {
            RemoteFunction::Scalar { input_maps, .. } => input_maps.as_ref(),
            RemoteFunction::Vector { input_maps, .. } => input_maps.as_ref(),
        }
    }

    pub fn tasks(&self) -> &[super::TaskExpression] {
        match self {
            RemoteFunction::Scalar { tasks, .. } => tasks,
            RemoteFunction::Vector { tasks, .. } => tasks,
        }
    }

    pub fn output(&self) -> &super::expression::Expression {
        match self {
            RemoteFunction::Scalar { output, .. } => output,
            RemoteFunction::Vector { output, .. } => output,
        }
    }

    pub fn output_length(
        &self,
    ) -> Option<&super::expression::WithExpression<u64>> {
        match self {
            RemoteFunction::Scalar { .. } => None,
            RemoteFunction::Vector { output_length, .. } => Some(output_length),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum InlineFunction {
    #[serde(rename = "scalar.function")]
    Scalar {
        #[serde(skip_serializing_if = "Option::is_none")]
        input_maps: Option<super::expression::InputMaps>,
        tasks: Vec<super::TaskExpression>,
        output: super::expression::Expression,
    },
    #[serde(rename = "vector.function")]
    Vector {
        #[serde(skip_serializing_if = "Option::is_none")]
        input_maps: Option<super::expression::InputMaps>,
        tasks: Vec<super::TaskExpression>,
        output: super::expression::Expression,
    },
}

impl InlineFunction {
    pub fn input_maps(&self) -> Option<&super::expression::InputMaps> {
        match self {
            InlineFunction::Scalar { input_maps, .. } => input_maps.as_ref(),
            InlineFunction::Vector { input_maps, .. } => input_maps.as_ref(),
        }
    }

    pub fn tasks(&self) -> &[super::TaskExpression] {
        match self {
            InlineFunction::Scalar { tasks, .. } => tasks,
            InlineFunction::Vector { tasks, .. } => tasks,
        }
    }

    pub fn output(&self) -> &super::expression::Expression {
        match self {
            InlineFunction::Scalar { output, .. } => output,
            InlineFunction::Vector { output, .. } => output,
        }
    }
}
