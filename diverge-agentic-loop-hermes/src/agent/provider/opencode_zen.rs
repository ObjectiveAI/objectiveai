//! OpenCode Zen.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// OpenCode Zen.
///
/// APPLICATION: the harness sets `OPENCODE_ZEN_API_KEY` to
/// [`api_key`](Self::api_key) in the gateway's process environment
/// before Hermes starts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Provider {
    /// The discriminator. Always `opencode-zen`.
    pub provider: OpencodeZen,
    /// The API key, applied as `OPENCODE_ZEN_API_KEY`.
    pub api_key: String,
}

/// [`Provider`]'s discriminator.
///
/// One variant, and the reason the [union](super::Provider) can be
/// untagged: no other provider's arguments can produce this value.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum OpencodeZen {
    #[default]
    OpencodeZen,
}
