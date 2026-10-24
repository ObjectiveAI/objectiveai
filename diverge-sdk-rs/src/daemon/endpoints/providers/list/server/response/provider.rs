//! One provider, as the daemon holds it.

use serde::{Deserialize, Serialize};

use crate::daemon::endpoints::providers::Status;
use crate::provider::endpoints::capacity;
use crate::shared::Identity;
use crate::shared::listing::Keyed;

/// One provider the daemon accepts: its identity, the address the
/// daemon dials it at when its configuration dials it, and where the
/// daemon stands with it now — and, connected, what the provider
/// answered its `info` ask.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Provider {
    /// The provider's identity: the `(issuer, subject)` its token
    /// establishes in the handshake.
    pub identity: Identity,
    /// The address the daemon dials it at, as its configuration writes
    /// it; absent for a provider the daemon only accepts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    /// Where the daemon stands with it now: see [`Status`].
    pub status: Status,
    /// The provider's latest answer to the daemon's `capacity` ask, as
    /// the daemon cached it: see
    /// [`Frame`](crate::provider::endpoints::capacity::server::response::Frame).
    /// Absent until the provider has answered once, and whenever the
    /// status is not `connected`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capacity: Option<capacity::server::response::Frame>,
}

/// Keyed by the identity.
impl Keyed for Provider {
    type Key = Identity;

    fn key(&self) -> Identity {
        self.identity.clone()
    }
}
