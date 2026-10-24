//! One peer the provider dials.

use serde::{Deserialize, Serialize};

/// A peer the provider dials, redialling whenever the connection ends:
/// where it is, and the issuer the provider presents as to it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Client {
    /// Where the peer listens: `host:port`, dialled as
    /// `ws://<address>/`, or a URL with its own scheme.
    pub address: String,
    /// The issuer the provider presents its token from: the document's
    /// `identities` names the provider's subject there. The peer
    /// answers from the same issuer, and its identity must be one the
    /// document's `accepted` lists.
    pub issuer: String,
}
