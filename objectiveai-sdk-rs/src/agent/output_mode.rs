//! Output mode configuration for vector completions.
//!
//! The output mode determines how the LLM is constrained to select from
//! a set of predefined responses during vector completion. This setting
//! is **only used for vector completions** and is ignored for agent completions.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The method used to constrain LLM output to valid response keys.
///
/// In vector completions, the model must select from a predefined set of
/// responses. This enum controls *how* that constraint is enforced.
///
/// **Note:** This setting is only relevant for vector completions and is
/// completely ignored for agent completions.
#[derive(
    Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Hash, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "agent.OutputMode")]
pub enum OutputMode {
    /// The model is instructed via the prompt to output a specific key.
    ///
    /// This is the default and most widely supported mode.
    #[schemars(title = "Instruction")]
    Instruction,
    /// A JSON schema response format is used with an enum of possible keys.
    ///
    /// Requires model support for structured JSON output.
    #[schemars(title = "JsonSchema")]
    JsonSchema,
    /// A forced tool call with an argument schema containing possible keys.
    ///
    /// Requires model support for tool/function calling.
    #[schemars(title = "ToolCall")]
    ToolCall,
}

impl std::default::Default for OutputMode {
    fn default() -> Self {
        OutputMode::Instruction
    }
}

impl From<super::openrouter::OutputMode> for OutputMode {
    fn from(mode: super::openrouter::OutputMode) -> Self {
        match mode {
            super::openrouter::OutputMode::Instruction => {
                OutputMode::Instruction
            }
            super::openrouter::OutputMode::JsonSchema => OutputMode::JsonSchema,
            super::openrouter::OutputMode::ToolCall => OutputMode::ToolCall,
        }
    }
}

impl From<super::claude_agent_sdk::OutputMode> for OutputMode {
    fn from(mode: super::claude_agent_sdk::OutputMode) -> Self {
        match mode {
            super::claude_agent_sdk::OutputMode::Instruction => {
                OutputMode::Instruction
            }
        }
    }
}

impl From<super::codex_sdk::OutputMode> for OutputMode {
    fn from(mode: super::codex_sdk::OutputMode) -> Self {
        match mode {
            super::codex_sdk::OutputMode::Instruction => {
                OutputMode::Instruction
            }
        }
    }
}

impl From<super::mock::OutputMode> for OutputMode {
    fn from(mode: super::mock::OutputMode) -> Self {
        match mode {
            super::mock::OutputMode::Instruction => OutputMode::Instruction,
            super::mock::OutputMode::JsonSchema => OutputMode::JsonSchema,
            super::mock::OutputMode::ToolCall => OutputMode::ToolCall,
        }
    }
}

impl From<super::script::OutputMode> for OutputMode {
    fn from(mode: super::script::OutputMode) -> Self {
        match mode {
            super::script::OutputMode::Instruction => OutputMode::Instruction,
        }
    }
}
