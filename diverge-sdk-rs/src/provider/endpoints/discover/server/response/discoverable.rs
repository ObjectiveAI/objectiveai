//! One discoverable connected identity, what it says of itself, and
//! where its connection is.

use std::net::IpAddr;

use serde::{Deserialize, Serialize};

use crate::shared::Identity;
use crate::shared::discover::Description;
use crate::shared::listing::Keyed;

/// An identity connected to the provider whose discover flag was `1`
/// or `2`, the description its authorization frame carried — none
/// under the flag `1` — and where its connection is. The rules:
///
/// 1. **The connection's.** Every member is the connection's, so none
///    changes while the identity is listed: an identity that connects
///    again is listed again.
/// 2. **The IP.** `ip` is the other end of the connection's socket:
///    the IP a peer that dialled the provider dialled from, or the IP
///    the provider reached a peer it dialled at.
/// 3. **The address.** `address` is the dial target the provider
///    dialled the peer at — a `host:port` or a URL — as the provider's
///    own configuration writes it, for a peer the provider dialled;
///    absent for a peer that dialled the provider. It is a dial target,
///    never an IP.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Discoverable {
    /// The identity, `(iss, sub)`.
    pub identity: Identity,
    /// What it says of itself, if anything.
    pub description: Option<Description>,
    /// The other end of its connection's socket: rule 2.
    pub ip: IpAddr,
    /// Where the provider dialled it, when it did: rule 3.
    pub address: Option<String>,
}

/// Keyed by the identity: an identity holds at most one connection to
/// a provider, so it is listed at most once.
impl Keyed for Discoverable {
    type Key = Identity;

    fn key(&self) -> Identity {
        self.identity.clone()
    }
}
