//! The ChatGPT Codex backend.

use serde::{Deserialize, Serialize};

/// The ChatGPT Codex backend, over external OAuth. Single-use
/// rotating refresh tokens, so the credential is a RESOURCE: the
/// caller provides its current state, the run rotates it, and the
/// rotated state is surfaced back to the caller instead of silently
/// burning their login.
///
/// APPLICATION: the harness writes
/// [`auth_resource`](Self::auth_resource) as the
/// `providers.openai-codex` entry of `$HERMES_HOME/auth.json`
/// before Hermes starts — provider selection rides the config the
/// harness already owns, never the file's `active_provider`. On
/// token refresh Hermes rewrites the entry in place; the rewritten
/// document is the resource's next state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Provider {
    /// The discriminator. Always `openai-codex`.
    pub provider: OpenaiCodex,
    /// The ChatGPT OAuth state, verbatim JSON — the
    /// `providers.openai-codex` entry exactly as the caller's own
    /// Hermes stores it. `tokens.access_token` and
    /// `tokens.refresh_token` are hard-required; expiry is read
    /// from the access token's own JWT claims, and the endpoint is
    /// Hermes's fixed default rather than part of the document.
    pub auth_resource: String,
}

/// [`Provider`]'s discriminator.
///
/// One variant, and the reason the [union](super::Provider) can be
/// untagged: no other provider's arguments can produce this value.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum OpenaiCodex {
    #[default]
    OpenaiCodex,
}
