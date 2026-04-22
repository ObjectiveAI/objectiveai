//! Output mode configuration for vector completions.

use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// The method used to constrain LLM output to valid response keys.
///
/// **Note:** This setting is only relevant for vector completions and is
/// completely ignored for agent completions.
#[derive(
    Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Hash, JsonSchema, arbitrary::Arbitrary,
)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "agent.claude_code.OutputMode")]
pub enum OutputMode {
    /// The model is instructed via the prompt to output a specific key.
    #[default]
    Instruction,
}
