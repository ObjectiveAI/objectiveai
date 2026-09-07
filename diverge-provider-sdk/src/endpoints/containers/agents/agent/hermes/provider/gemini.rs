//! Google Gemini.

use serde::{Deserialize, Serialize};

/// Google Gemini (AI Studio).
///
/// APPLICATION: the harness sets `GEMINI_API_KEY` to
/// [`api_key`](Self::api_key) in the gateway's process environment
/// before Hermes starts.
/// The harness must NOT set `GOOGLE_API_KEY`, which Hermes
/// checks first.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Provider {
    /// The discriminator. Always `gemini`.
    pub provider: Gemini,
    /// The API key, applied as `GEMINI_API_KEY`.
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
pub enum Gemini {
    #[default]
    Gemini,
}
