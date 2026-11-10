//! One peer the provider dials.

use diverge_sdk::shared::Identity;
use serde::{Deserialize, Serialize};

/// A peer the provider dials, redialling whenever the connection ends:
/// where it is, and who must answer there. The provider presents its
/// one `identity` to it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Client {
    /// Where the peer listens: `host:port`, dialled as
    /// `ws://<address>/`, or a URL with its own scheme.
    pub address: String,
    /// The identity that must answer at `address`: an answer from any
    /// other is closed in the handshake. The document's `incoming`
    /// has no say over it.
    pub identity: Identity,
}
