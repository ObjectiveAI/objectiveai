//! Tool choice configuration for chat completions.

use serde::{Deserialize, Serialize};

/// Controls how the model uses tools.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolChoice {
    /// The model will not call any tools.
    #[serde(rename = "none")]
    None,
    /// The model decides whether to call tools.
    #[serde(rename = "auto")]
    Auto,
    /// The model must call at least one tool.
    #[serde(rename = "required")]
    Required,
    /// The model must call a specific function.
    #[serde(untagged)]
    Function(ToolChoiceFunction),
}

/// Specifies a specific function the model must call.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolChoiceFunction {
    /// A specific function to call.
    Function {
        function: ToolChoiceFunctionFunction,
    },
}

/// The function name for a forced tool choice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolChoiceFunctionFunction {
    /// The name of the function to call.
    pub name: String,
}
