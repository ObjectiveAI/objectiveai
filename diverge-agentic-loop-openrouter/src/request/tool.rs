//! Tool/function definitions for chat completions.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// A tool that can be called by the model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Tool {
    /// A function tool.
    Function { function: FunctionTool },
}

/// A function tool definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
