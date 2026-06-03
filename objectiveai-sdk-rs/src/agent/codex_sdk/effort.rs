//! Effort settings for Codex SDK Agent output.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Reasoning-effort level for the Codex model. Maps to Codex's
/// `model_reasoning_effort` (see `openai_codex_sdk` `ModelReasoningEffort`).
///
/// `Medium` is the default and is normalized to `None` during preparation
/// for content-addressing stability.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "agent.codex_sdk.Effort")]
pub enum Effort {
    /// Minimal reasoning, fastest responses.
    #[schemars(title = "Minimal")]
    #[serde(rename = "minimal")]
    Minimal,
    /// Low reasoning effort.
    #[schemars(title = "Low")]
    #[serde(rename = "low")]
    Low,
    /// Balanced reasoning (default, normalized away during preparation).
    #[schemars(title = "Medium")]
    #[serde(rename = "medium")]
    #[default]
    Medium,
    /// High reasoning effort.
    #[schemars(title = "High")]
    #[serde(rename = "high")]
    High,
}

impl Effort {
    /// Normalizes effort for deterministic hashing. The default `Medium`
    /// value is normalized to `None`.
    pub fn prepare(self) -> Option<Self> {
        if let Effort::Medium = self {
            None
        } else {
            Some(self)
        }
    }

    /// Validates the effort setting (always succeeds).
    pub fn validate(&self) -> Result<(), String> {
        Ok(())
    }

    /// Returns the wire-string representation of the effort level.
    pub fn as_str(&self) -> &'static str {
        match self {
            Effort::Minimal => "minimal",
            Effort::Low => "low",
            Effort::Medium => "medium",
            Effort::High => "high",
        }
    }
}
