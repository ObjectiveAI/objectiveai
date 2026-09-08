//! Output verbosity.

use diverge_provider_sdk::endpoints::containers::agents::agent::openrouter;
use serde::Serialize;

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
)]
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
    /// Maximum verbosity, most detailed output possible.
    #[serde(rename = "max")]
    Max,
}

/// The provider request's verbosity, variant for variant.
impl From<openrouter::Verbosity> for Verbosity {
    fn from(verbosity: openrouter::Verbosity) -> Self {
        match verbosity {
            openrouter::Verbosity::Low => Verbosity::Low,
            openrouter::Verbosity::Medium => Verbosity::Medium,
            openrouter::Verbosity::High => Verbosity::High,
            openrouter::Verbosity::Max => Verbosity::Max,
        }
    }
}
