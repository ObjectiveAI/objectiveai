//! Reasoning summaries.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// How much of its reasoning the model says out loud: Codex's
/// `model_reasoning_summary`. What the run's reasoning chunks carry —
/// Codex streams the summary, never the raw reasoning, so this is
/// the whole of what a caller can see of the model thinking.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Summary {
    /// The model decides.
    Auto,
    /// Short.
    Concise,
    /// Long.
    Detailed,
    /// No summary: the reasoning chunks stay empty.
    None,
}
