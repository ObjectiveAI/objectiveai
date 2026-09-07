//! Vercel AI Gateway.

use serde::{Deserialize, Serialize};

/// Vercel AI Gateway.
///
/// APPLICATION: the harness sets `AI_GATEWAY_API_KEY` to
/// [`api_key`](Self::api_key) in the gateway's process environment
/// before Hermes starts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Provider {
    /// The discriminator. Always `ai-gateway`.
    pub provider: AiGateway,
    /// The API key, applied as `AI_GATEWAY_API_KEY`.
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
pub enum AiGateway {
    #[default]
    AiGateway,
}
