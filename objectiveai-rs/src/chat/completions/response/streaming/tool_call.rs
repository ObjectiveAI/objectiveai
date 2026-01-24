//! Tool call types for streaming responses.

use serde::{Deserialize, Serialize};

/// A tool call delta in a streaming response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// The index of this tool call.
    pub index: u64,
    /// The type of tool call (always "function").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<super::ToolCallType>,
    /// The unique ID of this tool call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// The function call details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<ToolCallFunction>,
}

impl ToolCall {
    /// Accumulates another tool call into this one.
    pub fn push(
        &mut self,
        ToolCall {
            r#type,
            id,
            function,
            ..
        }: &ToolCall,
    ) {
        if self.r#type.is_none() {
            self.r#type = r#type.clone();
        }
        if self.id.is_none() {
            self.id = id.clone();
        }
        match (&mut self.function, &function) {
            (Some(self_function), Some(other_function)) => {
                self_function.push(other_function);
            }
            (None, Some(other_function)) => {
                self.function = Some(other_function.clone());
            }
            _ => {}
        }
    }
}

/// The type of tool call.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub enum ToolCallType {
    /// A function call.
    #[serde(rename = "function")]
    #[default]
    Function,
}

/// Function call details in a streaming tool call.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolCallFunction {
    /// The function name (only present in the first delta).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The arguments being streamed (accumulated across deltas).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<String>,
}

impl ToolCallFunction {
    /// Accumulates another function call delta into this one.
    pub fn push(&mut self, other: &ToolCallFunction) {
        if self.name.is_none() {
            self.name = other.name.clone();
        }
        match (&mut self.arguments, &other.arguments) {
            (Some(self_arguments), Some(other_arguments)) => {
                self_arguments.push_str(other_arguments);
            }
            (None, Some(other_arguments)) => {
                self.arguments = Some(other_arguments.clone());
            }
            _ => {}
        }
    }
}
