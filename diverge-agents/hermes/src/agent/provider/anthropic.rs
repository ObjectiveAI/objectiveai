//! Anthropic.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Anthropic, over the Messages API.
///
/// APPLICATION: the harness sets `ANTHROPIC_API_KEY` to the
/// secret's value in the gateway's process environment before
/// Hermes starts.
///
/// SECRET: `ANTHROPIC_API_KEY`, fixed — the API key, under that name.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Provider {
    /// The discriminator. Always `anthropic`.
    pub provider: Anthropic,
}

/// [`Provider`]'s discriminator.
///
/// One variant, and the reason the [union](super::Provider) can be
/// untagged: no other provider's arguments can produce this value.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum Anthropic {
    #[default]
    Anthropic,
}
