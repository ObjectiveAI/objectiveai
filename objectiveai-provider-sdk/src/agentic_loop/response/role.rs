//! Message roles.

use serde::{Deserialize, Serialize};

/// The role of an assistant response — always `assistant`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AssistantRole {
    #[serde(rename = "assistant")]
    #[default]
    Assistant,
}

/// The role of a tool response — always `tool`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ToolRole {
    #[serde(rename = "tool")]
    #[default]
    Tool,
}
