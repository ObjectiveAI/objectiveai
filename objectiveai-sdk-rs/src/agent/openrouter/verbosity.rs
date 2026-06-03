//! Verbosity settings for Agent output.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The verbosity level for model output.
///
/// This setting hints to the model how detailed its responses should be.
/// Not all models support this parameter.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "agent.openrouter.Verbosity")]
pub enum Verbosity {
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
    /// Maximum verbosity, most detailed output possible.
    #[schemars(title = "Max")]
    #[serde(rename = "max")]
    Max,
}

impl Verbosity {
    /// Normalizes verbosity for deterministic hashing.
    ///
    /// The default `Medium` value is normalized to `None`.
    pub fn prepare(self) -> Option<Self> {
        if let Verbosity::Medium = self {
            None
        } else {
            Some(self)
        }
    }

    /// Validates the verbosity setting (always succeeds).
    pub fn validate(&self) -> Result<(), String> {
        Ok(())
    }
}
