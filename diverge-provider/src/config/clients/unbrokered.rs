//! One peer the provider dials in the unbrokered mode.

use serde::{Deserialize, Serialize};

/// One peer the provider dials, and what it presents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Unbrokered {
    /// Where the peer listens: `host:port`, the host a name the
    /// resolver answers for or an IP address, `[…]:port` for an IPv6
    /// address.
    pub address: String,
    /// The credential the provider presents as the connection's first
    /// frame.
    pub key: String,
    /// Who the peer is: the string every handler and every capability
    /// receives as the client's identity. Supplied, not learned; the
    /// provider chose whom to dial.
    pub identity: String,
}
