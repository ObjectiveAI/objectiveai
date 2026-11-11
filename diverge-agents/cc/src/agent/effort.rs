//! Reasoning effort.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// How much effort the model should spend before answering.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Effort {
    /// Least.
    Low,
    /// Balanced.
    #[default]
    Medium,
    /// More.
    High,
    /// Above high, below max.
    Xhigh,
    /// As much as the model will spend.
    Max,
}
