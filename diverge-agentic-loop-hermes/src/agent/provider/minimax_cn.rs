//! MiniMax.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// MiniMax, China endpoint.
///
/// APPLICATION: the harness sets `MINIMAX_CN_API_KEY` to the
/// secret's value in the gateway's process environment before
/// Hermes starts.
///
/// SECRET: `MINIMAX_CN_API_KEY`, fixed — the API key, under that name.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Provider {
    /// The discriminator. Always `minimax-cn`.
    pub provider: MinimaxCn,
}

/// [`Provider`]'s discriminator.
///
/// One variant, and the reason the [union](super::Provider) can be
/// untagged: no other provider's arguments can produce this value.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum MinimaxCn {
    #[default]
    MinimaxCn,
}
