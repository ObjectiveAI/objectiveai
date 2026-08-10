//! Output verbosity.

use serde::{Deserialize, Serialize};

/// How detailed the model's responses should be. A hint — not every
/// model honours it.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum Verbosity {
    /// Concise.
    Low,
    /// Balanced.
    #[default]
    Medium,
    /// Thorough.
    High,
    /// As detailed as the model will go.
    Max,
}
