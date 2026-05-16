//! Effort settings for Agent output.

use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// The effort level for model output.
///
/// This setting hints to the model how detailed its responses should be.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema, arbitrary::Arbitrary)]
#[schemars(rename = "agent.claude_agent_sdk.Effort")]
pub enum Effort {
    /// Minimal output, concise responses.
    #[schemars(title = "Low")]
    #[serde(rename = "low")]
    Low,
    /// Balanced output (default, normalized away during preparation).
    #[schemars(title = "Medium")]
    #[serde(rename = "medium")]
    Medium,
    /// Detailed output with thorough explanations.
    #[schemars(title = "High")]
    #[serde(rename = "high")]
    High,
    /// Maximum effort, most detailed output possible.
    #[schemars(title = "Max")]
    #[serde(rename = "max")]
    Max,
}

impl Effort {
    /// Normalizes effort for deterministic hashing.
    ///
    /// The default `Medium` value is normalized to `None`.
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

    /// Returns the string representation of the effort level.
    pub fn as_str(&self) -> &'static str {
        match self {
            Effort::Low => "low",
            Effort::Medium => "medium",
            Effort::High => "high",
            Effort::Max => "max",
        }
    }
}
