//! How OpenRouter output is constrained.

use serde::{Deserialize, Serialize};

/// How the model is constrained to select from a fixed set of
/// responses.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum OutputMode {
    /// Told in the prompt to output a specific key. The most widely
    /// supported, and the only one that needs nothing of the model.
    #[default]
    Instruction,
    /// A JSON schema response format enumerating the keys. Needs
    /// structured-output support.
    JsonSchema,
    /// A forced tool call whose argument schema enumerates the keys.
    /// Needs tool-calling support.
    ToolCall,
}
