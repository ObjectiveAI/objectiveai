//! Why a run ended unexpectedly.

use serde::{Deserialize, Serialize};

/// Why a run ended without anyone asking it to, as its
/// [`Undeployed`](super::Undeployed) says. Exactly one per unexpected
/// end; an expected end carries none.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    /// The connection to the provider the run was deployed on ended.
    ProviderLost,
    /// The provider was disabled, and its connection closed.
    ProviderDisabled,
    /// The provider's record was edited, and its connection closed.
    ProviderEdited,
    /// The daemon stopped.
    DaemonStopped,
    /// The provider ended the run with no word from the daemon: the
    /// container exited on its own.
    Exited,
    /// The run's stream failed; the `error` item kept before this one
    /// says how.
    Failed,
}
