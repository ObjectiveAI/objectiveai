//! xAI.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// xAI.
///
/// APPLICATION: the harness sets `XAI_API_KEY` to the
/// secret's value in the gateway's process environment before
/// Hermes starts.
///
/// SECRET: `XAI_API_KEY`, fixed — the API key, under that name. The
/// `x_search` toolset declares the same name; declared from both
/// places it is one secret.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Provider {
    /// The discriminator. Always `xai`.
    pub provider: Xai,
}

/// [`Provider`]'s discriminator.
///
/// One variant, and the reason the [union](super::Provider) can be
/// untagged: no other provider's arguments can produce this value.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum Xai {
    #[default]
    Xai,
}
