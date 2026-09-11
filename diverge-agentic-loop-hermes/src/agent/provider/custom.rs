//! Any OpenAI-compatible endpoint.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Any OpenAI-compatible endpoint the caller names — the local
/// runtimes' home (ollama, vllm, llama.cpp all alias onto it in
/// Hermes itself).
///
/// APPLICATION: the harness sets `CUSTOM_BASE_URL` to
/// [`base_url`](Self::base_url) in the gateway's process environment
/// before Hermes starts, and writes the same URL as `model.base_url`
/// in the config it already owns, beside `model.api_key` when
/// [`api_key`](Self::api_key) is present — Hermes reads that key only
/// next to a configured URL. With no key, Hermes supplies its own
/// `no-key-required` placeholder — the keyless case is the endpoint's
/// business, not a protocol gap.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Provider {
    /// The discriminator. Always `custom`.
    pub provider: Custom,
    /// The endpoint, applied as `CUSTOM_BASE_URL` and `model.base_url`.
    pub base_url: String,
    /// The API key the endpoint expects, if it expects one; applied
    /// as `model.api_key`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
}

/// [`Provider`]'s discriminator.
///
/// One variant, and the reason the [union](super::Provider) can be
/// untagged: no other provider's arguments can produce this value.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum Custom {
    #[default]
    Custom,
}
