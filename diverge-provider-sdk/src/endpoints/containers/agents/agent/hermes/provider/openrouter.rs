//! OpenRouter.

use serde::{Deserialize, Serialize};

/// OpenRouter.
///
/// APPLICATION: the harness sets `OPENROUTER_API_KEY` to
/// [`api_key`](Self::api_key) in the gateway's process environment
/// before Hermes starts.
/// Hermes discards keys that do not start with `sk-or-`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Provider {
    /// The discriminator. Always `openrouter`.
    pub provider: Openrouter,
    /// The API key, applied as `OPENROUTER_API_KEY`.
    pub api_key: String,
}

/// [`Provider`]'s discriminator.
///
/// One variant, and the reason the [union](super::Provider) can be
/// untagged: no other provider's arguments can produce this value.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum Openrouter {
    #[default]
    Openrouter,
}
