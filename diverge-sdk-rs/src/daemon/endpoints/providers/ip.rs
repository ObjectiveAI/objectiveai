//! A network IP, or a block of them.

use std::net::IpAddr;

use serde::{Deserialize, Serialize};

/// A network IP, or a block of them: what an incoming provider may dial
/// from, and what a remote daemon's route may be listed at. Tagged by
/// `type`. An IP is never a dial target: that is an address, a
/// `host:port` or a URL.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Ip {
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

impl Ip {
    /// Whether `ip` is this IP, or in this block. An IPv4 address
    /// mapped into IPv6 is read as the IPv4 address it maps.
    pub fn contains(&self, ip: IpAddr) -> bool {
        let ip = ip.to_canonical();
        match self {
            Ip::Ip { ip: own } => own.to_canonical() == ip,
            Ip::Cidr { cidr } => cidr.contains(&ip),
        }
    }
}
