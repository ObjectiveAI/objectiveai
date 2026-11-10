//! One peer the provider dials.

use serde::{Deserialize, Serialize};

/// A peer the provider dials, redialling whenever the connection ends:
/// where it is. The provider presents its one `identity` to it, and
/// the peer's own must be one the document's `accepted` lists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Client {
    /// Where the peer listens: `host:port`, dialled as
    /// `ws://<address>/`, or a URL with its own scheme.
    pub address: String,
}
