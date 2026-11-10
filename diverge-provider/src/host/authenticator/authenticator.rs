//! Who the provider is, and whom it serves.

use std::collections::HashSet;
use std::net::IpAddr;

use diverge_sdk::identities::Own;
use diverge_sdk::shared::Identity;
use diverge_sdk::wire::handshake::{Authenticator, Peer};
use futures_util::future::BoxFuture;

use super::Peers;

/// The provider's own identity, the identities that may dial it, and
/// which identities a connection holds now — see the [rules](super).
#[derive(Debug)]
pub struct ProviderAuthenticator {
    own: Own,
    incoming: HashSet<Identity>,
    peers: Peers,
}

impl ProviderAuthenticator {
    /// The provider as `own` presents it, dialled by `incoming`, none
    /// held.
    pub fn new(own: Own, incoming: impl IntoIterator<Item = Identity>) -> ProviderAuthenticator {
        ProviderAuthenticator { own, incoming: incoming.into_iter().collect(), peers: Peers::default() }
    }

    /// The authenticator for one handshake on a connection a peer
    /// dialled: the peer must be in `incoming` — rule 3.
    pub fn admitting(&self) -> Judging<'_> {
        Judging { authenticator: self, dialled: None }
    }

    /// The authenticator for one dial of the peer `identity`: the
    /// answer must be it — rule 3.
    pub fn dialling<'a>(&'a self, identity: &'a Identity) -> Judging<'a> {
        Judging { authenticator: self, dialled: Some(identity) }
    }

    /// The connection holding `identity` ended: it is let go — rule 4.
    pub async fn release(&self, identity: &Identity) {
        self.peers.release(identity).await;
    }
}

/// The provider's authenticator for one handshake: its own token, and
/// whom the other end must be.
#[derive(Debug, Clone, Copy)]
pub struct Judging<'a> {
    authenticator: &'a ProviderAuthenticator,
    /// The peer a dial expects to answer, for a dial; `None` on a
    /// connection a peer dialled.
    dialled: Option<&'a Identity>,
}

impl Authenticator for Judging<'_> {
    fn present<'a>(&'a self) -> BoxFuture<'a, Result<String, String>> {
        Box::pin(self.authenticator.own.present())
    }

    fn verify<'a>(&'a self, token: &'a str, _address: IpAddr) -> BoxFuture<'a, Result<Peer, String>> {
        Box::pin(async move {
            // Rule 2: whom it claims to be is judged first, and only
            // then is its issuer trusted and its keys fetched.
            let this = self.authenticator;
            let claimed = this.own.claimed(token)?;
            match self.dialled {
                Some(dialled) if dialled != &claimed => return Err(format!("{claimed} answered a dial of {dialled}")),
                None if !this.incoming.contains(&claimed) => return Err(format!("{claimed} is not an identity that may dial this provider")),
                _ => {}
            }
            let peer = this.own.verify_from(token, &claimed.issuer).await?;
            if peer != claimed {
                return Err(format!("{peer} is not {claimed}, whom the token claimed"));
            }
            if !this.peers.take(&peer).await {
                return Err(format!("{peer} is held by another connection"));
            }
            Ok(peer)
        })
    }

    fn abandon<'a>(&'a self, peer: &'a Peer) -> BoxFuture<'a, ()> {
        Box::pin(self.authenticator.peers.release(peer))
    }
}
