//! What an end supplies to the handshake.

use std::net::IpAddr;
use std::sync::Arc;

use futures_util::future::BoxFuture;

use super::Peer;
use crate::wire::frame::auth::{KEY_LEN, SIGNATURE_LEN};

/// What signs as an identity: the private half of the key the
/// identity's token names, kept wherever its holder keeps it. The
/// handshake asks it for one signature, and never for the key.
pub trait Signer: Send + Sync {
    /// The Ed25519 signature of `message`, RFC 8032.
    fn sign(&self, message: &[u8]) -> [u8; SIGNATURE_LEN];
}

/// What an end presents: its token, and what signs as the key the
/// token names. The two go together: a token presented with a signer
/// of another key proves nothing, and the other end closes.
#[derive(Clone)]
pub struct Presented {
    /// The token: an ID token naming the key `signer` holds.
    pub token: String,
    /// What signs with that key's private half.
    pub signer: Arc<dyn Signer>,
}

/// The other end, judged: the identity its token establishes, and the
/// key the token names, which the handshake then holds the other end
/// to.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Judged {
    /// The identity.
    pub peer: Peer,
    /// The Ed25519 public key the token names, in its `cnf` claim.
    pub key: [u8; KEY_LEN],
}

/// An end's own side of the handshake: its token and its key to
/// present, and its judgment of the other end's token.
///
/// One trait for both ends, because both do both: the connector
/// presents and then verifies, the connectee verifies and then
/// presents. Object-safe — its futures are boxed — so that what dials
/// again and again, a [`Link`](crate::wire::proxy::link::Link) or an
/// app's proxy, holds one as `Arc<dyn Authenticator>`.
///
/// An `Err` is a reason for the caller's log. It never reaches the
/// wire: a failed handshake is a close, and nothing else.
pub trait Authenticator: Send + Sync {
    /// This end's token, good for presenting now, and what signs as
    /// the key it names: those of the identity it connects as or
    /// answers as. `Err` when this end holds no such identity, or
    /// cannot get a token for it.
    fn present<'a>(&'a self) -> BoxFuture<'a, Result<Presented, String>>;

    /// The other end's `token`, presented from `address`, judged:
    /// `Ok` with the identity it establishes and the key it names, or
    /// `Err` with why it is not accepted. A token that names no key is
    /// not accepted. The handshake, not this, then checks that the
    /// other end holds the key.
    fn verify<'a>(&'a self, token: &'a str, address: IpAddr) -> BoxFuture<'a, Result<Judged, String>>;

    /// The handshake failed after [`verify`](Self::verify) accepted
    /// `peer`: whatever this end took for `peer` in accepting it — a
    /// lock that one connection at a time may hold — is let go. Called
    /// at most once per handshake, and only after `verify` answered
    /// `Ok`; a handshake that completes never calls it, and what
    /// `verify` took is then the caller's to let go when the connection
    /// ends. By default, nothing is taken and nothing is let go.
    fn abandon<'a>(&'a self, peer: &'a Peer) -> BoxFuture<'a, ()> {
        let _ = peer;
        Box::pin(async {})
    }
}
