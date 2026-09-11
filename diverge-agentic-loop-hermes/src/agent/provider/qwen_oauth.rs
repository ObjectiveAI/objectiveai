//! Qwen's portal.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Qwen's portal, over OAuth. Single-use rotating refresh tokens,
/// so the credential is a RESOURCE: the caller provides its current
/// state, the run rotates it, and the rotated state is surfaced
/// back to the caller instead of silently burning their login.
///
/// APPLICATION: unlike the other OAuth providers this state is not
/// Hermes's own — it is the Qwen CLI's token file. The document
/// lives in the vault under [`QWEN_OAUTH`](diverge_provider_sdk::shared::containers::vault::keys::QWEN_OAUTH); the
/// harness locks that key for the run, reads the document, and
/// writes it to `~/.qwen/oauth_creds.json` at the REAL home
/// directory (Hermes hardcodes that path; it ignores
/// `$HERMES_HOME`). On token refresh Hermes rewrites the creds file;
/// at the run's end the rewritten document is set back under the
/// key, and the key unlocked. Nothing about it is an argument:
/// choosing this provider is the whole ask.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Provider {
    /// The discriminator. Always `qwen-oauth`.
    pub provider: QwenOauth,
}

/// [`Provider`]'s discriminator.
///
/// One variant, and the reason the [union](super::Provider) can be
/// untagged: no other provider's arguments can produce this value.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum QwenOauth {
    #[default]
    QwenOauth,
}
