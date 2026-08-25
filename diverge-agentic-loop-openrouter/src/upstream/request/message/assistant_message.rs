//! Assistant messages and their tool calls.

use super::super::RichContent;
use serde::{Deserialize, Serialize};

/// An assistant message (model's previous response).
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
)]
pub struct AssistantMessage {
    /// The message content, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<RichContent>,
    /// Refusal message if the model declined to respond.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refusal: Option<String>,
    /// Tool calls made by the assistant.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<AssistantToolCall>>,
    /// Reasoning content from models that support chain-of-thought.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,
}

/// A tool call made by the assistant.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AssistantToolCall {
    /// A function call with an ID and function details.
    Function {
        /// The unique ID of this tool call.
        id: String,
        /// The function being called.
        function: AssistantToolCallFunction,
    },
}

/// Details of a function call made by the assistant.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
)]
pub struct AssistantToolCallFunction {
    /// The name of the function to call.
    pub name: String,
    /// The arguments to pass to the function, as a JSON string.
    pub arguments: String,
}

/// A tool call delta in a streaming response.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
)]
pub struct AssistantToolCallDelta {
    /// The index of this tool call.
    pub index: u64,
    /// The type of tool call (always "function").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<AssistantToolCallType>,
    /// The unique ID of this tool call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// The function call details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<AssistantToolCallFunctionDelta>,
}

/// The type of tool call.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Serialize,
    Deserialize,
    Default,
)]
pub enum AssistantToolCallType {
    /// A function call.
    #[serde(rename = "function")]
    #[default]
    Function,
}

/// Function call details in a streaming tool call.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    Default,
)]
pub struct AssistantToolCallFunctionDelta {
    /// The function name (only present in the first delta).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The arguments being streamed (accumulated across deltas).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<String>,
}
