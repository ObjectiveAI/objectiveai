//! A lister asking to see a container.

use std::net::IpAddr;

use serde::{Deserialize, Serialize};

/// Who asks to see the container, and from where.
///
/// Both claims are ATTESTED: the provider saw the address on the
/// lister's socket, and the identity is the one the lister's
/// connection was authorized under. Nothing here is the lister's own
/// word, which is the difference from an
/// [`Authorize`](super::Authorize), whose authorization the connector
/// wrote. A runner that answers yes lets the lister learn that this
/// container exists and what its id is, and nothing more: seeing a
/// container is not joining it, and a connect still asks on its own.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct List {
    /// Where the lister's socket comes from, as the provider sees it:
    /// the same signal, with the same caveats, as an
    /// [`Authorize`](super::Authorize)'s `address`.
    pub address: IpAddr,
    /// The identity the lister's connection was authorized under: the
    /// provider's word for the lister, as its
    /// [`Auth`](crate::wire::frame::auth::Auth) was judged.
    pub identity: String,
}
