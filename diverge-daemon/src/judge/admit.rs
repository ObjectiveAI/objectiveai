//! The handshake: a verified identity becomes an account, a provider,
//! or nothing.

use std::collections::HashSet;
use std::net::IpAddr;

use diverge_sdk::identities::Own;
use diverge_sdk::shared::Identity;
use diverge_sdk::wire::handshake::{Authenticator, Peer};
use futures_util::future::BoxFuture;

use super::Who;
use crate::store::{self, Store, accounts};

/// What a verified identity is to this daemon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Admitted {
    /// A client: the account the identity is.
    Client(Who),
    /// A provider the daemon works with.
    Provider(Identity),
}

/// The daemon's side of every handshake — with a client, a provider or
/// another daemon alike. The rules:
///
/// 1. **Presenting.** The daemon presents, at an issuer, the token of
///    the subject its `identities` names there — see [`Own`].
/// 2. **Verifying.** The other end's token is verified against its
///    issuer's keys, trusting exactly the issuers `identities` names.
/// 3. **Admitting.** A verified identity the configuration's
///    `providers.accepted` lists is a provider; else one that is an
///    account's is that account, a client; else it is refused, and the
///    handshake closes the connection with no frame. A daemon with no
///    account and no accepted provider admits nothing at all.
#[derive(Debug)]
pub struct DaemonAuthenticator {
    own: Own,
    providers: HashSet<Identity>,
    store: Store,
}

impl DaemonAuthenticator {
    /// The daemon as `own` presents it, working with the `providers`,
    /// admitting the accounts of `store`.
    pub fn new(own: Own, providers: impl IntoIterator<Item = Identity>, store: Store) -> DaemonAuthenticator {
        DaemonAuthenticator { own, providers: providers.into_iter().collect(), store }
    }

    /// What `identity` is to this daemon — see rule 3 — or `None`.
    pub async fn admit(&self, identity: &Identity) -> Result<Option<Admitted>, store::Error> {
        if self.providers.contains(identity) {
            return Ok(Some(Admitted::Provider(identity.clone())));
        }
        let mut conn = self.store.acquire().await?;
        Ok(accounts::by_identity(&mut conn, identity, false).await?.map(|account| Admitted::Client(Who::Account(account.id))))
    }

    /// Whether `identity` is a provider the daemon works with.
    pub fn provider(&self, identity: &Identity) -> bool {
        self.providers.contains(identity)
    }
}

impl Authenticator for DaemonAuthenticator {
    fn present<'a>(&'a self, issuer: &'a str) -> BoxFuture<'a, Result<String, String>> {
        Box::pin(self.own.present(issuer))
    }

    fn verify<'a>(&'a self, token: &'a str, _address: IpAddr) -> BoxFuture<'a, Result<Peer, String>> {
        Box::pin(async move {
            let peer = self.own.verify(token).await?;
            match self.admit(&peer).await {
                Ok(Some(_)) => Ok(peer),
                Ok(None) => Err(format!("{peer} is neither an account nor a provider of this daemon's")),
                Err(error) => Err(error.to_string()),
            }
        })
    }
}
