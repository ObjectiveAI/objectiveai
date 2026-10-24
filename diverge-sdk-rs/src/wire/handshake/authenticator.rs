//! What an end supplies to the handshake.

use std::net::IpAddr;

use futures_util::future::BoxFuture;

use super::Peer;

/// An end's own side of the handshake: its token to present, and its
/// judgment of the other end's.
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
    /// This end's token from `issuer`, good for presenting now. `Err`
    /// when this end holds no identity at that issuer, or cannot get a
    /// token from it.
    fn present<'a>(&'a self, issuer: &'a str) -> BoxFuture<'a, Result<String, String>>;

    /// The other end's `token`, presented from `address`, judged:
    /// `Ok` with the identity it establishes, or `Err` with why it is
    /// not accepted.
    fn verify<'a>(&'a self, token: &'a str, address: IpAddr) -> BoxFuture<'a, Result<Peer, String>>;
}
