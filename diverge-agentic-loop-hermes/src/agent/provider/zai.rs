//! Z.AI.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Z.AI (GLM / Zhipu).
///
/// APPLICATION: the harness sets `GLM_API_KEY` to
/// [`api_key`](Self::api_key) in the gateway's process environment
/// before Hermes starts, and `GLM_BASE_URL` to
/// [`base_url`](Self::base_url) when present.
///
/// The optional endpoint is the one such override in the vocabulary,
/// and it earns its place: without a pinned base URL, Hermes's first
/// use fires a live probe across four candidate endpoints and caches
/// the winner to disk — a network round-trip at start, guessing at a
/// fact the caller knows (which of Z.AI's global and CN surfaces
/// their key belongs to).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Provider {
    /// The discriminator. Always `zai`.
    pub provider: Zai,
    /// The API key, applied as `GLM_API_KEY`.
    pub api_key: String,
    /// The endpoint the key belongs to, applied as `GLM_BASE_URL`;
    /// absent = Hermes's own endpoint probe.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
}

/// [`Provider`]'s discriminator.
///
/// One variant, and the reason the [union](super::Provider) can be
/// untagged: no other provider's arguments can produce this value.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum Zai {
    #[default]
    Zai,
}
