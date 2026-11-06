//! How a provider's connection is made: it dials the daemon, or the
//! daemon dials it.

use std::net::IpAddr;

use serde::{Deserialize, Serialize};

/// Which end dials, and what that end needs: exactly one of the two for
/// every provider. Tagged by `direction`, its members beside the
/// record's own. The rules:
///
/// 1. **Exactly one direction.** An incoming provider is never dialled.
///    An outgoing provider that dials the daemon is refused, whatever
///    address it dials from.
/// 2. **Incoming addresses.** An incoming provider's connection is
///    admitted only from a source address one of its `addresses`
///    contains, checked against the socket before its token is
///    verified. Absent, any address.
/// 3. **Outgoing identity.** An outgoing provider is dialled at its
///    `address`, presenting the daemon's token from the provider's own
///    issuer, and the identity that answers must be the record's; any
///    other is refused. Where the address resolves is not checked.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(tag = "direction", rename_all = "snake_case")]
pub enum Direction {
    /// The provider dials the daemon.
    Incoming {
        /// The source addresses it may dial from: rule 2. Absent, any.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        addresses: Option<Vec<Address>>,
    },
    /// The daemon dials the provider.
    Outgoing {
        /// Where the provider listens: `host:port`, dialled as
        /// `ws://<address>/`, or a URL with its own scheme.
        address: String,
    },
}

/// One source address an incoming provider may dial from: a single IP,
/// or a block of them. Tagged by `type`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Address {
    /// Exactly this IP.
    Ip {
        /// The IP, v4 or v6.
        ip: IpAddr,
    },
    /// Every IP in this block.
    Cidr {
        /// The block, as `10.0.0.0/24` or `2001:db8::/32`.
        #[schemars(with = "String")]
        cidr: ipnet::IpNet,
    },
}

impl Address {
    /// Whether `ip` is this address, or in this block. An IPv4 address
    /// mapped into IPv6 is read as the IPv4 address it maps.
    pub fn contains(&self, ip: IpAddr) -> bool {
        let ip = ip.to_canonical();
        match self {
            Address::Ip { ip: own } => own.to_canonical() == ip,
            Address::Cidr { cidr } => cidr.contains(&ip),
        }
    }
}

impl Direction {
    /// Whether a connection from `ip` is admitted: rule 2 for an
    /// incoming provider, and never for an outgoing one — rule 1.
    pub fn admits(&self, ip: IpAddr) -> bool {
        match self {
            Direction::Incoming { addresses: None } => true,
            Direction::Incoming { addresses: Some(addresses) } => addresses.iter().any(|address| address.contains(ip)),
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
