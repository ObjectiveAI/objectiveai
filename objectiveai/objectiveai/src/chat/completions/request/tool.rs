//! Tool/function definitions for chat completions.

use crate::functions;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Utilities for working with tool lists.
pub mod tools {
    /// Computes a content-addressed ID for a list of tools.
    pub fn id(tools: &[super::Tool]) -> String {
        let mut hasher = twox_hash::XxHash3_128::with_seed(0);
        hasher.write(serde_json::to_string(tools).unwrap().as_bytes());
        format!("{:0>22}", base62::encode(hasher.finish_128()))
    }
}

/// A tool that can be called by the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Tool {
    /// A function tool.
    Function { function: FunctionTool },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolExpression {
    Function {
        function: functions::expression::WithExpression<FunctionToolExpression>,
    },
}

impl ToolExpression {
    pub fn compile(
        self,
        params: &functions::expression::Params,
    ) -> Result<Tool, functions::expression::ExpressionError> {
        match self {
            ToolExpression::Function { function } => Ok(Tool::Function {
                function: function.compile_one(params)?.compile(params)?,
            }),
        }
    }
}

/// A function tool definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionTool {
    /// The name of the function.
    pub name: String,
    /// A description of what the function does.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// JSON Schema for the function parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Map<String, serde_json::Value>>,
    /// Whether to enforce strict schema validation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionToolExpression {
    pub name: functions::expression::WithExpression<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description:
        Option<functions::expression::WithExpression<Option<String>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<
        functions::expression::WithExpression<
            Option<
                IndexMap<
                    String,
                    functions::expression::WithExpression<ValueExpression>,
                >,
            >,
        >,
    >,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<functions::expression::WithExpression<Option<bool>>>,
}

impl FunctionToolExpression {
    pub fn compile(
        self,
        params: &functions::expression::Params,
    ) -> Result<FunctionTool, functions::expression::ExpressionError> {
        let name = self.name.compile_one(params)?;
        let description = self
            .description
            .map(|description| description.compile_one(params))
            .transpose()?
            .flatten();
        let parameters = self
            .parameters
            .map(|parameters| parameters.compile_one(params))
            .transpose()?
            .flatten()
            .map(|parameters| {
                let mut compiled_parameters =
                    serde_json::Map::with_capacity(parameters.len());
                for (key, value) in parameters {
                    compiled_parameters.insert(
                        key,
                        value.compile_one(params)?.compile(params)?,
                    );
                }
                Ok::<_, functions::expression::ExpressionError>(
                    compiled_parameters,
                )
            })
            .transpose()?;
        let strict = self
            .strict
            .map(|strict| strict.compile_one(params))
            .transpose()?
            .flatten();
        Ok(FunctionTool {
            name,
            description,
            parameters,
            strict,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ValueExpression {
    Null,
    Bool(bool),
    Number(serde_json::Number),
    String(String),
    Array(Vec<functions::expression::WithExpression<ValueExpression>>),
    Object(
        IndexMap<
            String,
            functions::expression::WithExpression<ValueExpression>,
        >,
    ),
}

impl ValueExpression {
    pub fn compile(
        self,
        params: &functions::expression::Params,
    ) -> Result<serde_json::Value, functions::expression::ExpressionError> {
        match self {
            ValueExpression::Null => Ok(serde_json::Value::Null),
            ValueExpression::Bool(bool) => Ok(serde_json::Value::Bool(bool)),
            ValueExpression::Number(number) => {
                Ok(serde_json::Value::Number(number))
            }
            ValueExpression::String(string) => {
                Ok(serde_json::Value::String(string))
            }
            ValueExpression::Array(array) => {
                let mut compiled_array = Vec::with_capacity(array.len());
                for item in array {
                    match item.compile_one_or_many(params)? {
                        functions::expression::OneOrMany::One(one_item) => {
                            compiled_array.push(one_item.compile(params)?);
                        }
                        functions::expression::OneOrMany::Many(many_items) => {
                            for item in many_items {
                                compiled_array.push(item.compile(params)?);
                            }
                        }
                    }
                }
                Ok(serde_json::Value::Array(compiled_array))
            }
            ValueExpression::Object(object) => {
                let mut compiled_object =
                    serde_json::Map::with_capacity(object.len());
                for (key, value) in object {
                    compiled_object.insert(
                        key,
                        value.compile_one(params)?.compile(params)?,
                    );
                }
                Ok(serde_json::Value::Object(compiled_object))
            }
        }
    }
}
