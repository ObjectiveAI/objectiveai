//! Who the provider is, and whom it serves.

use std::collections::HashSet;
use std::net::IpAddr;

use diverge_sdk::identities::Own;
use diverge_sdk::shared::Identity;
use diverge_sdk::wire::handshake::{Authenticator, Peer};
use futures_util::future::BoxFuture;

use super::Peers;

/// The provider's own identities, the identities it serves, and which
/// of them a connection holds now — see the [rules](super).
#[derive(Debug)]
pub struct ProviderAuthenticator {
    own: Own,
    accepted: HashSet<Identity>,
    peers: Peers,
}

impl ProviderAuthenticator {
    /// The provider as `own` presents it, serving `accepted`, none of
    /// them held.
    pub fn new(own: Own, accepted: impl IntoIterator<Item = Identity>) -> ProviderAuthenticator {
        ProviderAuthenticator { own, accepted: accepted.into_iter().collect(), peers: Peers::default() }
    }

    /// The connection holding `identity` ended: it is let go — rule 4.
    pub async fn release(&self, identity: &Identity) {
        self.peers.release(identity).await;
    }
}

impl Authenticator for ProviderAuthenticator {
    fn present<'a>(&'a self, issuer: &'a str) -> BoxFuture<'a, Result<String, String>> {
        Box::pin(self.own.present(issuer))
    }

    fn verify<'a>(&'a self, token: &'a str, _address: IpAddr) -> BoxFuture<'a, Result<Peer, String>> {
        Box::pin(async move {
            let peer = self.own.verify(token).await?;
            if !self.accepted.contains(&peer) {
                return Err(format!("{peer} is not an identity this provider serves"));
            }
            if !self.peers.take(&peer).await {
                return Err(format!("{peer} is held by another connection"));
            }
            Ok(peer)
        })
    }

    fn abandon<'a>(&'a self, peer: &'a Peer) -> BoxFuture<'a, ()> {
        Box::pin(self.peers.release(peer))
    }
}
