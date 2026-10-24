//! Who the provider is, and whom it serves.

use std::collections::HashSet;
use std::net::IpAddr;

use diverge_sdk::identities::Own;
use diverge_sdk::shared::Identity;
use diverge_sdk::wire::handshake::{Authenticator, Peer};
use futures_util::future::BoxFuture;

/// The provider's own identities and the identities it serves — see
/// the [rules](super).
#[derive(Debug)]
pub struct ProviderAuthenticator {
    own: Own,
    accepted: HashSet<Identity>,
}

impl ProviderAuthenticator {
    /// The provider as `own` presents it, serving `accepted`.
    pub fn new(own: Own, accepted: impl IntoIterator<Item = Identity>) -> ProviderAuthenticator {
        ProviderAuthenticator { own, accepted: accepted.into_iter().collect() }
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
            Ok(peer)
        })
    }
}
