//! Output verbosity.

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
