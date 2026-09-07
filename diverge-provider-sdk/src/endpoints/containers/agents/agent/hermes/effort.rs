//! How much reasoning to spend.

use serde::{Deserialize, Serialize};

/// Hermes's reasoning ladder, whole:
/// `none < minimal < low < medium < high < xhigh < max < ultra`.
///
/// Deliberately NOT the five-tier `Effort` the claude_code and
/// codex agents share: a different upstream owns a different
/// vocabulary — [`none`](Self::None) and [`ultra`](Self::Ultra)
/// exist only here, `none` being Hermes's explicit
/// reasoning-disabled sentinel (which is why the agent's field is
/// an `Option`: absent means the provider's default, `none` means
/// off, and those are different facts). Hermes clamps to each
/// wire's supported subset itself, always to the nearest WEAKER
/// level — a caller never has to know which rungs a given model
/// exposes.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum Effort {
    /// Reasoning explicitly disabled.
    None,
    /// Least.
    Minimal,
    /// Less.
    Low,
    /// Balanced.
    #[default]
    Medium,
    /// More.
    High,
    /// Above high, below max.
    Xhigh,
    /// As much as the model will spend.
    Max,
    /// Above even max, where a wire supports it.
    Ultra,
}
