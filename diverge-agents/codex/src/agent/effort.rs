//! Reasoning effort.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// How much effort the model spends before answering: Codex's
/// `model_reasoning_effort`, its five rungs as Codex spells them.
/// There is no `max`: `xhigh` is the top, and a model that has fewer
/// rungs takes the nearest it has.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Effort {
    /// Least.
    Minimal,
    /// Little.
    Low,
    /// Balanced.
    Medium,
    /// More.
    High,
    /// As much as the model will spend.
    Xhigh,
}
