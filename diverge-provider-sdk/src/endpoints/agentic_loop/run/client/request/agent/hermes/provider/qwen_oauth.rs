//! Qwen's portal.

use serde::{Deserialize, Serialize};

/// Qwen's portal, over OAuth. Single-use rotating refresh tokens,
/// so the credential is a RESOURCE: the caller provides its current
/// state, the run rotates it, and the rotated state is surfaced
/// back to the caller instead of silently burning their login.
///
/// APPLICATION: unlike the other OAuth providers this state is not
/// Hermes's own — it is the Qwen CLI's token file. The harness
/// fetches the resource's bytes and writes them to
/// `~/.qwen/oauth_creds.json` at the REAL home directory (Hermes
/// hardcodes that path; it ignores `$HERMES_HOME`), and itself adds
/// the minimal `providers.qwen-oauth` selection marker to
/// `$HERMES_HOME/auth.json` — the marker carries no token material
/// and is not the caller's to supply. On token refresh Hermes
/// rewrites the creds file; the rewritten document is the
/// resource's next state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Provider {
    /// The discriminator. Always `qwen-oauth`.
    pub provider: QwenOauth,
    /// The OAuth state's size-bearing identity — the FILE grammar,
    /// `f1:<size>:<base64url sha256 of the bytes>` — never the
    /// bytes themselves. The bytes are the Qwen CLI's
    /// `oauth_creds.json` document, JSON; they live with the client
    /// and arrive over the
    /// [`FetchResource`](crate::endpoints::agentic_loop::run::server::channel_request::Frame::FetchResource)
    /// exchange when the provider does not hold them. The document:
    /// `access_token`, `refresh_token`, `token_type`,
    /// `resource_url` and `expiry_date` (epoch MILLISECONDS — a
    /// missing or stale value forces a refresh on first use).
    /// Hermes rewrites the file with exactly those five keys, so
    /// anything extra in the document does not survive the first
    /// rotation.
    pub oauth_creds_resource: String,
}

/// [`Provider`]'s discriminator.
///
/// One variant, and the reason the [union](super::Provider) can be
/// untagged: no other provider's arguments can produce this value.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum QwenOauth {
    #[default]
    QwenOauth,
}
