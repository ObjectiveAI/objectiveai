//! How Codex SDK output is constrained.

use serde::{Deserialize, Serialize};

/// How the model is constrained to select from a fixed set of
/// responses.
///
/// One variant: this upstream drives an agent harness rather than a
/// raw completion, so schema-constrained and forced-tool-call modes
/// are not available to it.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum OutputMode {
    /// Told in the prompt to output a specific key.
    #[default]
    Instruction,
}
