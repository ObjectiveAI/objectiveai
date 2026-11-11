//! How a provider's connection is made: it dials the daemon, or the
//! daemon dials it.

use std::net::IpAddr;

use serde::{Deserialize, Serialize};

use super::Ip;

/// Which end dials, and what that end needs: exactly one of the two for
/// every provider. Tagged by `direction`, its members beside the
/// record's own. The rules:
///
/// 1. **Exactly one direction.** An incoming provider is never dialled.
///    An outgoing provider that dials the daemon is refused, whatever
///    IP it dials from.
/// 2. **Incoming IPs.** An incoming provider's connection is admitted
///    only from a source IP one of its `ips` contains, checked against
///    the socket before its token is verified. Absent, any IP; empty,
///    none.
/// 3. **Outgoing identity.** An outgoing provider is dialled at its
///    `address`, presenting the daemon's own token — its one
///    identity's — and the identity that answers must be the record's; any
///    other is refused. Where the address resolves is not checked.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(tag = "direction", rename_all = "snake_case")]
pub enum Direction {
    /// The provider dials the daemon.
    Incoming {
        /// The source IPs it may dial from: rule 2. Absent, any.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ips: Option<Vec<Ip>>,
    },
    /// The daemon dials the provider.
    Outgoing {
        /// Where the provider listens: `host:port`, dialled as
        /// `ws://<address>/`, or a URL with its own scheme.
        address: String,
    },
}

impl Direction {
    /// Whether a connection from `ip` is admitted: rule 2 for an
    /// incoming provider, and never for an outgoing one — rule 1.
    pub fn admits(&self, ip: IpAddr) -> bool {
        match self {
            Direction::Incoming { ips: None } => true,
            Direction::Incoming { ips: Some(ips) } => ips.iter().any(|allowed| allowed.contains(ip)),
            Direction::Outgoing { .. } => false,
        }
    }

    /// The address the daemon dials, for an outgoing provider.
    pub fn dialled(&self) -> Option<&str> {
        match self {
            Direction::Outgoing { address } => Some(address),
            Direction::Incoming { .. } => None,
        }
    }
}
