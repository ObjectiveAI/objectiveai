//! Tool call types for chat completion responses.

use crate::chat::completions::response;
use serde::{Deserialize, Serialize};

/// A tool call made by the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolCall {
    /// A function call.
    Function {
        /// Unique identifier for this tool call.
        id: String,
        /// The function being called.
        function: ToolCallFunction,
    },
}

impl Default for ToolCall {
    fn default() -> Self {
        ToolCall::Function {
            id: String::new(),
            function: ToolCallFunction::default(),
        }
    }
}

impl From<response::streaming::ToolCall> for ToolCall {
    fn from(
        response::streaming::ToolCall {
            id,
            function,
            r#type,
            ..
        }: response::streaming::ToolCall,
    ) -> Self {
        match r#type {
            Some(response::streaming::ToolCallType::Function) | None => {
                ToolCall::Function {
                    id: id.unwrap_or_default(),
                    function: function
                        .map(ToolCallFunction::from)
                        .unwrap_or_default(),
                }
            }
        }
    }
}

/// A function call with its name and arguments.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolCallFunction {
    /// The name of the function to call.
    pub name: String,
    /// The arguments to pass, as a JSON string.
    pub arguments: String,
}

impl From<response::streaming::ToolCallFunction> for ToolCallFunction {
    fn from(
        response::streaming::ToolCallFunction {
            name,
            arguments,
        } : response::streaming::ToolCallFunction,
    ) -> Self {
        Self {
            name: name.unwrap_or_default(),
            arguments: arguments.unwrap_or_default(),
        }
    }
}
