//! One announced connected identity, its announcement, and where its
//! connection is.

use std::net::IpAddr;

use serde::{Deserialize, Serialize};

use crate::shared::Identity;
use crate::shared::description::Description;
use crate::shared::listing::Keyed;

/// An identity connected to the provider that holds a
/// [`daemons::announce`](crate::provider::endpoints::daemons::announce)
/// open, the announcement that scope carries, and where its connection
/// is. The rules:
///
/// 1. **Fixed while listed.** Every member but `listening` is fixed
///    for as long as the identity is listed: `ip` and `address` are
///    the connection's, and `announcement` is the announce scope's. An
///    identity that announces again — on the same connection or on a
///    new one — is listed again.
/// 2. **The IP.** `ip` is the other end of the connection's socket:
///    the IP a peer that dialled the provider dialled from, or the IP
///    the provider reached a peer it dialled at.
/// 3. **The address.** `address` is the dial target the provider
///    dialled the peer at — a `host:port` or a URL — as the provider's
///    own configuration writes it, for a peer the provider dialled;
///    absent for a peer that dialled the provider. It is a dial target,
///    never an IP.
/// 4. **Listening.** `listening` is whether the identity has a
///    [`daemons::listen`](crate::provider::endpoints::daemons::listen)
///    open as the item is sent. It is the one member that changes while
///    the identity is listed, and a listing tells its change by the
///    identity alone — see [`Frame`](super::Frame).
/// 5. **The announcement.** `announcement` is the text of the
///    identity's open announce scope, exactly as its request carried
///    it. An identity is listed only while it holds that scope, so an
///    item always has one.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Listed {
    /// The identity, `(iss, sub)`.
    pub identity: Identity,
    /// The text it announced itself with: rule 5.
    pub announcement: Description,
    /// The other end of its connection's socket: rule 2.
    pub ip: IpAddr,
    /// Where the provider dialled it, when it did: rule 3.
    pub address: Option<String>,
    /// Whether it listens for messages: rule 4.
    pub listening: bool,
}

/// Keyed by the identity: an identity holds at most one connection to
/// a provider, and a connection at most one announce scope, so it is
/// listed at most once.
impl Keyed for Listed {
    type Key = Identity;

    fn key(&self) -> Identity {
        self.identity.clone()
    }
}
