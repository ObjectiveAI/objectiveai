//! Reasoning configuration.

use serde::{Deserialize, Serialize};

/// How much the model should reason before answering, and how much of
/// that reasoning to report.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Reasoning {
    /// Whether to reason at all.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// A token budget for reasoning. Mutually exclusive with
    /// [`effort`](Self::effort) on most models — a budget and a level
    /// are two ways of saying the same thing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u64>,
    /// How hard to reason, as a level rather than a budget.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<ReasoningEffort>,
    /// How much of the reasoning to summarise back.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary_verbosity: Option<ReasoningSummaryVerbosity>,
}

/// How hard the model should reason.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningEffort {
    /// Do not reason.
    #[default]
    None,
    Minimal,
    Low,
    Medium,
    High,
    Xhigh,
}

/// How much of the reasoning to summarise in the response.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningSummaryVerbosity {
    /// Let the model decide.
    #[default]
    Auto,
    /// Brief.
    Concise,
    /// Thorough.
    Detailed,
}
