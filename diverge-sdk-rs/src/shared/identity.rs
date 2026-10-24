//! Who a party is.

/// A party's identity: the `(issuer, subject)` its token establishes —
/// a daemon's account, a provider, a daemon a provider serves. The
/// handshake's [`Peer`](crate::wire::handshake::Peer), by the name the
/// endpoints use for it.
pub type Identity = crate::wire::handshake::Peer;
