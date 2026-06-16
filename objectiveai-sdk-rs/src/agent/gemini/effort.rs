//! Effort settings for Gemini Agent output.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The effort level for model output.
///
/// This setting hints to the model how detailed its responses should be.
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
#[schemars(rename = "agent.gemini.Effort")]
pub enum Effort {
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
            Effort::Low => "low",
            Effort::Medium => "medium",
            Effort::High => "high",
        }
    }
}
