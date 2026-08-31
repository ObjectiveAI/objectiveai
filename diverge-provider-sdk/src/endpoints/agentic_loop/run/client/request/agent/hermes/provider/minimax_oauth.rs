//! MiniMax over OAuth.

use serde::{Deserialize, Serialize};

/// MiniMax over OAuth. Single-use rotating refresh tokens, so the
/// credential is a RESOURCE: the caller provides its current state,
/// the run rotates it, and the rotated state is surfaced back to
/// the caller instead of silently burning their login. (Keyed
/// MiniMax is [`minimax`](super::minimax::Provider) /
/// [`minimax_cn`](super::minimax_cn::Provider).)
///
/// APPLICATION: the harness writes
/// [`auth_resource`](Self::auth_resource) as the
/// `providers.minimax-oauth` entry of `$HERMES_HOME/auth.json`
/// before Hermes starts — provider selection rides the config the
/// harness already owns, never the file's `active_provider`. On
/// token refresh Hermes rewrites the entry in place; the rewritten
/// document is the resource's next state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Provider {
    /// The discriminator. Always `minimax-oauth`.
    pub provider: MinimaxOauth,
    /// The MiniMax OAuth state, verbatim JSON — the
    /// `providers.minimax-oauth` entry exactly as the caller's own
    /// Hermes stores it. This one must arrive COMPLETE:
    /// `access_token`, `refresh_token`, `portal_base_url`,
    /// `inference_base_url`, `client_id` and `expires_at` are all
    /// indexed unconditionally once a refresh fires — and Hermes
    /// attempts one on first use.
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
pub enum MinimaxOauth {
    #[default]
    MinimaxOauth,
}
