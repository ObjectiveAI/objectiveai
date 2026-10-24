//! The handshake: a verified identity becomes an account, a provider,
//! a remote daemon, or nothing.

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

/// Who the other end of one handshake must be: what the connection is
/// for decides it, and nothing else.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Expected<'a> {
    /// A socket the daemon accepted: a provider the configuration
    /// accepts, or an account.
    Admitted,
    /// A connection another daemon opened through a provider: an
    /// account, and never a provider.
    Account,
    /// A provider the daemon dialled: one the configuration accepts.
    Provider,
    /// A remote daemon the daemon reached through a provider: exactly
    /// the identity the configuration's link names it by there.
    Daemon(&'a Identity),
}

/// The daemon's side of every handshake — with a client, a provider or
/// another daemon alike, accepted or dialled. The rules, the same for
/// every connection:
///
/// 1. **Presenting.** The daemon presents, at an issuer, the token of
///    the subject its `identities` names there — see [`Own`].
/// 2. **Verifying.** The other end's token is verified against its
///    issuer's keys, trusting exactly the issuers `identities` names.
/// 3. **Expecting.** The verified identity must be who the connection
///    [`Expected`]s, from the configuration or the accounts: an
///    accepted socket's, a provider the configuration accepts or an
///    account; a relayed connection's, an account; a dialled
///    provider's, one the configuration accepts; a remote daemon's,
///    the identity its link names. Anyone else is refused inside the
///    handshake, which closes the connection with no frame — whichever
///    end dialled. A daemon with no account and no accepted provider
///    admits nothing at all.
///
/// [`expecting`](Self::expecting) is the authenticator one handshake
/// is made with; the daemon itself is none, so that no handshake is
/// made without saying who it expects.
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

    /// The authenticator for one handshake expecting `expected` — see
    /// rule 3.
    pub fn expecting<'a>(&'a self, expected: Expected<'a>) -> Judging<'a> {
        Judging { authenticator: self, expected }
    }

    /// What `identity` is to this daemon on an accepted socket: an
    /// accepted provider, else the account it is, else `None`.
    pub async fn admit(&self, identity: &Identity) -> Result<Option<Admitted>, store::Error> {
        if self.providers.contains(identity) {
            return Ok(Some(Admitted::Provider(identity.clone())));
        }
        self.account(identity).await
    }

    /// The account `identity` is, as a client, or `None`.
    async fn account(&self, identity: &Identity) -> Result<Option<Admitted>, store::Error> {
        let mut conn = self.store.acquire().await?;
        Ok(accounts::by_identity(&mut conn, identity, false).await?.map(|account| Admitted::Client(Who::Account(account.id))))
    }

    /// Whether `peer` is who `expected` says — rule 3.
    async fn holds(&self, expected: Expected<'_>, peer: &Peer) -> Result<bool, store::Error> {
        Ok(match expected {
            Expected::Admitted => self.admit(peer).await?.is_some(),
            Expected::Account => self.account(peer).await?.is_some(),
            Expected::Provider => self.providers.contains(peer),
            Expected::Daemon(identity) => peer == identity,
        })
    }
}

/// The daemon's authenticator for one handshake: its own tokens, and
/// the one identity, or kind of identity, the other end must be.
#[derive(Debug, Clone, Copy)]
pub struct Judging<'a> {
    authenticator: &'a DaemonAuthenticator,
    expected: Expected<'a>,
}

impl Authenticator for Judging<'_> {
    fn present<'a>(&'a self, issuer: &'a str) -> BoxFuture<'a, Result<String, String>> {
        Box::pin(self.authenticator.own.present(issuer))
    }

    fn verify<'a>(&'a self, token: &'a str, _address: IpAddr) -> BoxFuture<'a, Result<Peer, String>> {
        Box::pin(async move {
            let peer = self.authenticator.own.verify(token).await?;
            match self.authenticator.holds(self.expected, &peer).await {
                Ok(true) => Ok(peer),
                Ok(false) => Err(format!("{peer} is not who the connection expects: {:?}", self.expected)),
                Err(error) => Err(error.to_string()),
            }
        })
    }
}
