//! OpenCode's free tier.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// OpenCode's free tier — keyless by design.
///
/// APPLICATION: nothing. Hermes short-circuits this provider to an
/// anonymous placeholder before any credential check, so the marker
/// is the whole argument.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Provider {
    /// The discriminator. Always `opencode-free`.
    pub provider: OpencodeFree,
}

/// [`Provider`]'s discriminator.
///
/// One variant, and the reason the [union](super::Provider) can be
/// untagged: no other provider's arguments can produce this value.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum OpencodeFree {
    #[default]
    OpencodeFree,
}
