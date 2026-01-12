use crate::chat;
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Task {
    #[serde(rename = "scalar.function")]
    ScalarFunction(ScalarFunctionTask),
    #[serde(rename = "vector.function")]
    VectorFunction(VectorFunctionTask),
    #[serde(rename = "vector.completion")]
    VectorCompletion(VectorCompletionTask),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalarFunctionTaskExpression {
    pub owner: String,
    pub repository: String,
    pub commit: String,

    // receives input
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip: Option<super::expression::Expression>,

    // indexes into maps
    #[serde(skip_serializing_if = "Option::is_none")]
    pub map: Option<u64>,

    // receives input + maps
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalarFunctionTask {
    pub owner: String,
    pub repository: String,
    pub commit: String,
    pub input: super::expression::Input,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorFunctionTaskExpression {
    pub owner: String,
    pub repository: String,
    pub commit: String,

    // receives input
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip: Option<super::expression::Expression>,

    // indexes into maps
    #[serde(skip_serializing_if = "Option::is_none")]
    pub map: Option<u64>,

    // receives input + maps
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorFunctionTask {
    pub owner: String,
    pub repository: String,
    pub commit: String,
    pub input: super::expression::Input,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorCompletionTaskExpression {
    // receives input
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip: Option<super::expression::Expression>,

    // indexes into maps
    #[serde(skip_serializing_if = "Option::is_none")]
    pub map: Option<u64>,

    // receives input + maps
    pub messages: super::expression::WithExpression<
        Vec<
            super::expression::WithExpression<
                chat::completions::request::MessageExpression,
            >,
        >,
    >,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorCompletionTask {
    pub messages: Vec<chat::completions::request::Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<chat::completions::request::Tool>>,
    pub responses: Vec<chat::completions::request::RichContent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CompiledTask {
    One(Task),       // no map field
    Many(Vec<Task>), // mapped
}
