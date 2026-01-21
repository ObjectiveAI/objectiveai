//! Verbosity settings for Ensemble LLM output.

use serde::{Deserialize, Serialize};

/// The verbosity level for model output.
///
/// This setting hints to the model how detailed its responses should be.
/// Not all models support this parameter.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Verbosity {
    /// Minimal output, concise responses.
    #[serde(rename = "low")]
    Low,
    /// Balanced output (default, normalized away during preparation).
    #[serde(rename = "medium")]
    Medium,
    /// Detailed output with thorough explanations.
    #[serde(rename = "high")]
    High,
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
