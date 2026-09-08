//! Reasoning/thinking configuration.

use crate::agent;
use serde::Serialize;

/// Configuration for model reasoning/thinking capabilities.
///
/// Some models (like o1, o3, Claude with extended thinking) support
/// explicit reasoning modes where they can "think" before responding.
/// This struct configures those capabilities.
///
/// **Note:** The `max_tokens`, `effort`, and `summary_verbosity` fields are
/// only supported by some models. Unsupported fields are silently ignored.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Serialize,
)]
pub struct Reasoning {
    /// Whether reasoning is enabled. Defaults to `true` if other fields are set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// Maximum tokens for the reasoning/thinking output.
    ///
    /// Only supported by some models.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u64>,
    /// The reasoning effort level.
    ///
    /// Only supported by some models.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<ReasoningEffort>,
    /// Verbosity of reasoning summaries in the response.
    ///
    /// Only supported by some models.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary_verbosity: Option<ReasoningSummaryVerbosity>,
}

/// The level of effort the model should put into reasoning.
///
/// Only supported by some models.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Serialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningEffort {
    /// No reasoning.
    None,
    /// Minimal reasoning effort.
    Minimal,
    /// Low reasoning effort.
    Low,
    /// Medium reasoning effort.
    Medium,
    /// High reasoning effort.
    High,
    /// Maximum reasoning effort.
    Xhigh,
}

/// Verbosity of the reasoning summary included in responses.
///
/// Only supported by some models.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Serialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningSummaryVerbosity {
    /// Let the model decide (default, normalized away).
    Auto,
    /// Brief summary of reasoning.
    Concise,
    /// Thorough summary of reasoning.
    Detailed,
}

/// The provider request's reasoning configuration, field for field.
impl From<agent::Reasoning> for Reasoning {
    fn from(reasoning: agent::Reasoning) -> Self {
        Reasoning {
            enabled: reasoning.enabled,
            max_tokens: reasoning.max_tokens,
            effort: reasoning.effort.map(Into::into),
            summary_verbosity: reasoning.summary_verbosity.map(Into::into),
        }
    }
}

impl From<agent::ReasoningEffort> for ReasoningEffort {
    fn from(effort: agent::ReasoningEffort) -> Self {
        match effort {
            agent::ReasoningEffort::None => ReasoningEffort::None,
            agent::ReasoningEffort::Minimal => ReasoningEffort::Minimal,
            agent::ReasoningEffort::Low => ReasoningEffort::Low,
            agent::ReasoningEffort::Medium => ReasoningEffort::Medium,
            agent::ReasoningEffort::High => ReasoningEffort::High,
            agent::ReasoningEffort::Xhigh => ReasoningEffort::Xhigh,
        }
    }
}

impl From<agent::ReasoningSummaryVerbosity> for ReasoningSummaryVerbosity {
    fn from(verbosity: agent::ReasoningSummaryVerbosity) -> Self {
        match verbosity {
            agent::ReasoningSummaryVerbosity::Auto => {
                ReasoningSummaryVerbosity::Auto
            }
            agent::ReasoningSummaryVerbosity::Concise => {
                ReasoningSummaryVerbosity::Concise
            }
            agent::ReasoningSummaryVerbosity::Detailed => {
                ReasoningSummaryVerbosity::Detailed
            }
        }
    }
}
