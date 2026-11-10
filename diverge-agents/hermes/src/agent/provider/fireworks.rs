//! Fireworks AI.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Fireworks AI.
///
/// APPLICATION: the harness sets `FIREWORKS_API_KEY` to the
/// secret's value in the gateway's process environment before
/// Hermes starts.
///
/// SECRET: `FIREWORKS_API_KEY`, fixed — the API key, under that name.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Provider {
    /// The discriminator. Always `fireworks`.
    pub provider: Fireworks,
}

/// [`Provider`]'s discriminator.
///
/// One variant, and the reason the [union](super::Provider) can be
/// untagged: no other provider's arguments can produce this value.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum Fireworks {
    #[default]
    Fireworks,
}
