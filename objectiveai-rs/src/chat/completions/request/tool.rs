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

/// Expression variant of [`Tool`] for dynamic content.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolExpression {
    /// A function tool expression.
    Function {
        /// The function definition expression.
        function: functions::expression::WithExpression<FunctionToolExpression>,
    },
}

impl ToolExpression {
    /// Compiles the expression into a concrete [`Tool`].
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

/// Expression variant of [`FunctionTool`] for dynamic content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionToolExpression {
    /// The function name expression.
    pub name: functions::expression::WithExpression<String>,
    /// The description expression.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description:
        Option<functions::expression::WithExpression<Option<String>>>,
    /// The parameters expression.
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
    /// The strict mode expression.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<functions::expression::WithExpression<Option<bool>>>,
}

impl FunctionToolExpression {
    /// Compiles the expression into a concrete [`FunctionTool`].
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

/// A JSON value expression for dynamic content.
///
/// This allows JSON values to contain JMESPath expressions that are
/// evaluated at runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ValueExpression {
    /// A null value.
    Null,
    /// A boolean value.
    Bool(bool),
    /// A numeric value.
    Number(serde_json::Number),
    /// A string value.
    String(String),
    /// An array of value expressions.
    Array(Vec<functions::expression::WithExpression<ValueExpression>>),
    /// An object with value expressions.
    Object(
        IndexMap<
            String,
            functions::expression::WithExpression<ValueExpression>,
        >,
    ),
}

impl ValueExpression {
    /// Compiles the expression into a concrete JSON value.
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
