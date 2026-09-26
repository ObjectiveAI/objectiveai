//! The ChatGPT Codex backend.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The ChatGPT Codex backend, over external OAuth. Single-use
/// rotating refresh tokens, so the credential is a RESOURCE: the
/// caller provides its current state, the run rotates it, and the
/// rotated state is surfaced back to the caller instead of silently
/// burning their login.
///
/// APPLICATION: the document lives in the vault under
/// [`OPENAI_CODEX_OAUTH`](diverge_sdk::shared::containers::vault::keys::OPENAI_CODEX_OAUTH). The harness
/// locks that key for the run, reads the document, and writes it as
/// the `providers.openai-codex` entry of `$HERMES_HOME/auth.json`
/// before Hermes starts — provider selection rides the config the
/// harness already owns, never the file's `active_provider`. On
/// token refresh Hermes rewrites the entry in place; at the run's
/// end the rewritten document is set back under the key, and the
/// key unlocked. Nothing about it is an argument: choosing this
/// provider is the whole ask.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Provider {
    /// The discriminator. Always `openai-codex`.
    pub provider: OpenaiCodex,
}

/// [`Provider`]'s discriminator.
///
/// One variant, and the reason the [union](super::Provider) can be
/// untagged: no other provider's arguments can produce this value.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum OpenaiCodex {
    #[default]
    OpenaiCodex,
}
