//! MiniMax over OAuth.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// MiniMax over OAuth. Single-use rotating refresh tokens, so the
/// credential is a RESOURCE: the caller provides its current state,
/// the run rotates it, and the rotated state is surfaced back to
/// the caller instead of silently burning their login. (Keyed
/// MiniMax is [`minimax`](super::minimax::Provider) /
/// [`minimax_cn`](super::minimax_cn::Provider).)
///
/// APPLICATION: the document lives in the vault under
/// [`MINIMAX_OAUTH`](diverge_provider_sdk::shared::containers::vault::keys::MINIMAX_OAUTH). The harness locks that
/// key for the run, reads the document, and writes it as the
/// `providers.minimax-oauth` entry of `$HERMES_HOME/auth.json`
/// before Hermes starts — provider selection rides the config the
/// harness already owns, never the file's `active_provider`. On
/// token refresh Hermes rewrites the entry in place; at the run's
/// end the rewritten document is set back under the key, and the
/// key unlocked. Nothing about it is an argument: choosing this
/// provider is the whole ask.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Provider {
    /// The discriminator. Always `minimax-oauth`.
    pub provider: MinimaxOauth,
}

/// [`Provider`]'s discriminator.
///
/// One variant, and the reason the [union](super::Provider) can be
/// untagged: no other provider's arguments can produce this value.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum MinimaxOauth {
    #[default]
    MinimaxOauth,
}
