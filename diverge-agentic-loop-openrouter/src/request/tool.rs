//! Tool/function definitions for chat completions.

use indexmap::IndexMap;
use serde::Serialize;

/// A tool that can be called by the model.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Tool {
    /// A function tool.
    Function { function: FunctionTool },
}

/// A function tool definition.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FunctionTool {
    /// The name of the function.
    pub name: String,
    /// A description of what the function does.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// JSON Schema for the function parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<IndexMap<String, serde_json::Value>>,
    /// Whether to enforce strict schema validation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

impl Tool {
    /// An MCP tool, as the function tool OpenRouter takes.
    ///
    /// The name is the tool's own — the proxy already presents final
    /// names — and the parameters are the tool's input schema,
    /// entry for entry. What rmcp carries beyond that (output
    /// schema, annotations, icons) has no home in a function tool
    /// and is dropped.
    pub fn new(tool: rmcp::model::Tool) -> Self {
        Tool::Function {
            function: FunctionTool {
                name: tool.name.into_owned(),
                description: tool
                    .description
                    .map(|description| description.into_owned()),
                parameters: Some(
                    tool.input_schema
                        .iter()
                        .map(|(key, value)| (key.clone(), value.clone()))
                        .collect(),
                ),
                strict: Some(true),
            },
        }
    }
}
