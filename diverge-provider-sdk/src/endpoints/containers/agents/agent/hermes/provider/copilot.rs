//! GitHub Copilot.

use serde::{Deserialize, Serialize};

/// GitHub Copilot (token exchange).
///
/// APPLICATION: the harness sets `COPILOT_GITHUB_TOKEN` to
/// [`github_token`](Self::github_token) in the gateway's process
/// environment before Hermes starts; Hermes performs the
/// GitHub-token → Copilot-api-token exchange on its own, and setting
/// the var also suppresses its `gh auth token` fallback.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Provider {
    /// The discriminator. Always `copilot`.
    pub provider: Copilot,
    /// The GitHub token the exchange starts from, applied as
    /// `COPILOT_GITHUB_TOKEN`. Must be an OAuth or fine-grained
    /// token (`gho_`/`ghu_`/`github_pat_`) — Hermes rejects classic
    /// `ghp_` PATs.
    pub github_token: String,
}

/// [`Provider`]'s discriminator.
///
/// One variant, and the reason the [union](super::Provider) can be
/// untagged: no other provider's arguments can produce this value.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum Copilot {
    #[default]
    Copilot,
}
