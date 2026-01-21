//! Task types for Function definitions.
//!
//! Tasks are the building blocks of Functions. Each task either calls another
//! Function or runs a Vector Completion. Tasks can be conditionally skipped
//! or mapped over arrays of inputs.

use crate::chat;
use serde::{Deserialize, Serialize};

/// A task definition with JMESPath expressions (pre-compilation).
///
/// Task expressions contain dynamic fields that are resolved against input
/// data during compilation. Use [`compile`](Self::compile) to produce a
/// concrete [`Task`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TaskExpression {
    #[serde(rename = "scalar.function")]
    ScalarFunction(ScalarFunctionTaskExpression),
    #[serde(rename = "vector.function")]
    VectorFunction(VectorFunctionTaskExpression),
    #[serde(rename = "vector.completion")]
    VectorCompletion(VectorCompletionTaskExpression),
}

impl TaskExpression {
    pub fn take_skip(&mut self) -> Option<super::expression::Expression> {
        match self {
            TaskExpression::ScalarFunction(task) => task.skip.take(),
            TaskExpression::VectorFunction(task) => task.skip.take(),
            TaskExpression::VectorCompletion(task) => task.skip.take(),
        }
    }

    pub fn input_map(&self) -> Option<u64> {
        match self {
            TaskExpression::ScalarFunction(task) => task.map,
            TaskExpression::VectorFunction(task) => task.map,
            TaskExpression::VectorCompletion(task) => task.map,
        }
    }

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
        }
    }
}

/// A compiled task ready for execution.
///
/// Produced by compiling a [`TaskExpression`] against input data. All
/// JMESPath expressions have been resolved to concrete values.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Task {
    /// Calls a scalar function (produces a single score).
    #[serde(rename = "scalar.function")]
    ScalarFunction(ScalarFunctionTask),
    /// Calls a vector function (produces a vector of scores).
    #[serde(rename = "vector.function")]
    VectorFunction(VectorFunctionTask),
    /// Runs a vector completion.
    #[serde(rename = "vector.completion")]
    VectorCompletion(VectorCompletionTask),
}

/// Expression for a task that calls a scalar function (pre-compilation).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalarFunctionTaskExpression {
    /// GitHub repository owner.
    pub owner: String,
    /// GitHub repository name.
    pub repository: String,
    /// Git commit SHA for the function version.
    pub commit: String,

    /// If this expression evaluates to true, skip the task. Receives: `input`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip: Option<super::expression::Expression>,

    /// Index into `input_maps` for mapped execution. If set, this task is
    /// expanded into multiple instances.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub map: Option<u64>,

    /// Expression for the input to pass to the function.
    /// Receives: `input`, `map` (if mapped).
    pub input:
        super::expression::WithExpression<super::expression::InputExpression>,
}

impl ScalarFunctionTaskExpression {
    pub fn compile(
        self,
        params: &super::expression::Params,
    ) -> Result<ScalarFunctionTask, super::expression::ExpressionError> {
        let input = self.input.compile_one(params)?.compile(params)?;
        Ok(ScalarFunctionTask {
            owner: self.owner,
            repository: self.repository,
            commit: self.commit,
            input,
        })
    }
}

/// A compiled scalar function task ready for execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalarFunctionTask {
    /// GitHub repository owner.
    pub owner: String,
    /// GitHub repository name.
    pub repository: String,
    /// Git commit SHA for the function version.
    pub commit: String,
    /// The resolved input to pass to the function.
    pub input: super::expression::Input,
}

/// Expression for a task that calls a vector function (pre-compilation).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorFunctionTaskExpression {
    /// GitHub repository owner.
    pub owner: String,
    /// GitHub repository name.
    pub repository: String,
    /// Git commit SHA for the function version.
    pub commit: String,

    /// If this expression evaluates to true, skip the task. Receives: `input`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip: Option<super::expression::Expression>,

    /// Index into `input_maps` for mapped execution. If set, this task is
    /// expanded into multiple instances.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub map: Option<u64>,

    /// Expression for the input to pass to the function.
    /// Receives: `input`, `map` (if mapped).
    pub input:
        super::expression::WithExpression<super::expression::InputExpression>,
}

impl VectorFunctionTaskExpression {
    pub fn compile(
        self,
        params: &super::expression::Params,
    ) -> Result<VectorFunctionTask, super::expression::ExpressionError> {
        let input = self.input.compile_one(params)?.compile(params)?;
        Ok(VectorFunctionTask {
            owner: self.owner,
            repository: self.repository,
            commit: self.commit,
            input,
        })
    }
}

/// A compiled vector function task ready for execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorFunctionTask {
    /// GitHub repository owner.
    pub owner: String,
    /// GitHub repository name.
    pub repository: String,
    /// Git commit SHA for the function version.
    pub commit: String,
    /// The resolved input to pass to the function.
    pub input: super::expression::Input,
}

/// Expression for a task that runs a vector completion (pre-compilation).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorCompletionTaskExpression {
    /// If this expression evaluates to true, skip the task. Receives: `input`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip: Option<super::expression::Expression>,

    /// Index into `input_maps` for mapped execution. If set, this task is
    /// expanded into multiple instances.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub map: Option<u64>,

    /// Expression for the conversation messages (the prompt).
    /// Receives: `input`, `map` (if mapped).
    pub messages: super::expression::WithExpression<
        Vec<
            super::expression::WithExpression<
                chat::completions::request::MessageExpression,
            >,
        >,
    >,
    /// Expression for tools available to the completion (read-only context).
    /// Receives: `input`, `map` (if mapped).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<
        super::expression::WithExpression<
            Option<
                Vec<
                    super::expression::WithExpression<
                        chat::completions::request::ToolExpression,
                    >,
                >,
            >,
        >,
    >,
    /// Expression for the possible responses the LLMs can vote for.
    /// Receives: `input`, `map` (if mapped).
    pub responses: super::expression::WithExpression<
        Vec<
            super::expression::WithExpression<
                chat::completions::request::RichContentExpression,
            >,
        >,
    >,
}

impl VectorCompletionTaskExpression {
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

        // compile tools
        let tools = self
            .tools
            .map(|tools| tools.compile_one(params))
            .transpose()?
            .flatten()
            .map(|tools| {
                let mut compiled_tools = Vec::with_capacity(tools.len());
                for tool in tools {
                    match tool.compile_one_or_many(params)? {
                        super::expression::OneOrMany::One(one_tool) => {
                            compiled_tools.push(one_tool.compile(params)?);
                        }
                        super::expression::OneOrMany::Many(many_tools) => {
                            for tool in many_tools {
                                compiled_tools.push(tool.compile(params)?);
                            }
                        }
                    }
                }
                Ok::<_, super::expression::ExpressionError>(compiled_tools)
            })
            .transpose()?;

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
            tools,
            responses: compiled_responses,
        })
    }
}

/// A compiled vector completion task ready for execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorCompletionTask {
    /// The resolved conversation messages.
    pub messages: Vec<chat::completions::request::Message>,
    /// The resolved tools (read-only context for the completion).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<chat::completions::request::Tool>>,
    /// The resolved response options the LLMs can vote for.
    pub responses: Vec<chat::completions::request::RichContent>,
}

/// The result of compiling a task expression.
///
/// Tasks without a `map` field compile to a single task. Tasks with a `map`
/// field are expanded into multiple tasks, one per element in the referenced
/// input map sub-array.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CompiledTask {
    /// A single task (no mapping).
    One(Task),
    /// Multiple task instances from mapped execution.
    Many(Vec<Task>),
}
