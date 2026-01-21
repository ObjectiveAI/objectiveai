//! Reasoning configuration for models that support extended thinking.

use serde::{Deserialize, Serialize};

/// Configuration for model reasoning/thinking capabilities.
///
/// Some models (like o1, o3, Claude with extended thinking) support
/// explicit reasoning modes where they can "think" before responding.
/// This struct configures those capabilities.
///
/// **Note:** The `max_tokens`, `effort`, and `summary_verbosity` fields are
/// only supported by some models. Unsupported fields are silently ignored.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
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

impl Reasoning {
    pub fn prepare(mut self) -> Option<Self> {
        if let Some(0) = self.max_tokens {
            self.max_tokens = None;
        }
        if let Some(ReasoningSummaryVerbosity::Auto) = self.summary_verbosity {
            self.summary_verbosity = None;
        }
        if self.enabled.is_none()
            && (self.max_tokens.is_some()
                || self.effort.is_some()
                || self.summary_verbosity.is_some())
        {
            self.enabled = Some(true);
        }
        if self.enabled.is_some()
            || self.max_tokens.is_some()
            || self.effort.is_some()
            || self.summary_verbosity.is_some()
        {
            Some(self)
        } else {
            None
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self
            .max_tokens
            .is_some_and(|max_tokens| max_tokens > i32::MAX as u64)
        {
            Err(format!(
                "`reasoning.max_tokens` must be at most {}",
                i32::MAX
            ))
        } else if let Some(false) = self.enabled {
            if self.max_tokens.is_some() {
                Err(
                    "`reasoning.enabled` cannot be false if `reasoning.max_tokens` is set"
                        .to_string(),
                )
            } else if self.effort.is_some() {
                Err("`reasoning.enabled` cannot be false if `reasoning.effort` is set".to_string())
            } else if self.summary_verbosity.is_some() {
                Err(
                    "`reasoning.enabled` cannot be false if `reasoning.summary_verbosity` is set"
                        .to_string(),
                )
            } else {
                Ok(())
            }
        } else {
            Ok(())
        }
    }
}

/// The level of effort the model should put into reasoning.
///
/// Only supported by some models.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningSummaryVerbosity {
    /// Let the model decide (default, normalized away).
    Auto,
    /// Brief summary of reasoning.
    Concise,
    /// Thorough summary of reasoning.
    Detailed,
}
