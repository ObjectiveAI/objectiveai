//! Nous Research.

use serde::{Deserialize, Serialize};

/// Nous Research — the Portal, Hermes's home team. OAuth with
/// single-use rotating refresh tokens, so the credential is a
/// RESOURCE: the caller provides its current state, the run rotates
/// it, and the rotated state is surfaced back to the caller instead
/// of silently burning their login.
///
/// APPLICATION: the harness writes
/// [`auth_resource`](Self::auth_resource) as the `providers.nous`
/// entry of `$HERMES_HOME/auth.json` before Hermes starts —
/// provider selection rides the config the harness already owns,
/// never the file's `active_provider`, which Hermes treats as a
/// last-resort fallback. On token refresh Hermes rewrites the entry
/// in place; the rewritten document is the resource's next state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Provider {
    /// The discriminator. Always `nous`.
    pub provider: Nous,
    /// The Nous Portal OAuth state, verbatim JSON — the
    /// `providers.nous` entry exactly as the caller's own Hermes
    /// stores it. `access_token` and `refresh_token` are required;
    /// the rest defaults. Hermes host-allowlists the portal and
    /// inference URLs, healing anything else back to production —
    /// a staging host cannot ride this document.
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
pub enum Nous {
    #[default]
    Nous,
}
