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
    Clone,
    Copy,
    Debug,
    Default,
    Serialize,
    Deserialize,
    PartialEq,
    Hash,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "agent.mock.OutputMode")]
pub enum OutputMode {
    /// The model is instructed via the prompt to output a specific key.
    ///
    /// This is the default and most widely supported mode.
    #[schemars(title = "Instruction")]
    #[default]
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
