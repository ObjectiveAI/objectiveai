//! Qwen's portal.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Qwen's portal, over OAuth. Single-use rotating refresh tokens,
/// so the credential is a ROTATING secret: the caller holds its
/// current state, the run rotates it, and the rotated state is
/// collected back instead of silently burning their login.
///
/// APPLICATION: unlike the other OAuth providers this state is not
/// Hermes's own — it is the Qwen CLI's token file. The harness
/// writes the fed document to `~/.qwen/oauth_creds.json` at the REAL
/// home directory (Hermes hardcodes that path; it ignores
/// `$HERMES_HOME`) before Hermes starts. On token refresh Hermes
/// rewrites the creds file; after the gateway exits the file is read
/// back as the run left it and handed to the proxy's collect.
/// Nothing about it is an argument: choosing this provider is the
/// whole ask.
///
/// SECRET: `QWEN_OAUTH`, rotating — the CLI's token document, a JSON
/// object, exactly as the file holds it.
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
