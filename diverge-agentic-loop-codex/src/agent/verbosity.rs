//! Verbosity.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// How long the model's answers run: Codex's `model_verbosity`,
/// honored by the models that take it (the reference names the GPT-5
/// family on the Responses API) and unset otherwise.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Verbosity {
    /// Terse.
    Low,
    /// Balanced.
    Medium,
    /// Expansive.
    High,
}
