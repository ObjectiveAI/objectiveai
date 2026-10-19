//! Nous Research.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Nous Research — the Portal, Hermes's home team. OAuth with
/// single-use rotating refresh tokens, so the credential is a
/// ROTATING secret: the caller holds its current state, the run
/// rotates it, and the rotated state is collected back instead of
/// silently burning their login.
///
/// APPLICATION: the harness writes the fed document as the
/// `providers.nous` entry of `$HERMES_HOME/auth.json` before Hermes
/// starts — provider selection rides the config the harness already
/// owns, never the file's `active_provider`, which Hermes treats as
/// a last-resort fallback. On token refresh Hermes rewrites the
/// entry in place; after the gateway exits the entry is read back as
/// the run left it and handed to the proxy's collect. Nothing about
/// it is an argument: choosing this provider is the whole ask.
///
/// SECRET: `NOUS_OAUTH`, rotating — the OAuth state document, a JSON
/// object, exactly as Hermes keeps the entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Provider {
    /// The discriminator. Always `nous`.
    pub provider: Nous,
}

/// [`Provider`]'s discriminator.
///
/// One variant, and the reason the [union](super::Provider) can be
/// untagged: no other provider's arguments can produce this value.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum Nous {
    #[default]
    Nous,
}
