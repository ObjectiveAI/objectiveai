//! Anthropic.

use serde::{Deserialize, Serialize};

/// Anthropic, over the Messages API.
///
/// APPLICATION: the harness sets `ANTHROPIC_API_KEY` to
/// [`api_key`](Self::api_key) in the gateway's process environment
/// before Hermes starts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Provider {
    /// The discriminator. Always `anthropic`.
    pub provider: Anthropic,
    /// The API key, applied as `ANTHROPIC_API_KEY`.
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
pub enum Anthropic {
    #[default]
    Anthropic,
}
