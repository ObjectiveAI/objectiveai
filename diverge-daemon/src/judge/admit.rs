//! The handshake: a verified identity becomes an account, a provider,
//! a remote daemon, or nothing.

use std::collections::HashSet;
use std::net::IpAddr;

use diverge_sdk::identities::Own;
use diverge_sdk::shared::Identity;
use diverge_sdk::wire::handshake::{Authenticator, Peer};
use futures_util::future::BoxFuture;
use tokio::sync::Mutex;

use super::Who;
use crate::store::{self, Store, accounts};

/// What a verified identity is to this daemon on a socket it accepted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Admitted {
    /// A client: the account the identity is.
    Client(Who),
    /// A provider the daemon works with.
    Provider(Identity),
}

/// Which connection a handshake is for: what decides the list the
/// other end must be on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Expected {
    /// A socket the daemon accepted: a provider on `providers.accepted`,
    /// or an account.
    Admitted,
    /// A provider the daemon dialled: one on `providers.accepted`.
    Provider,
    /// Another daemon, through a provider — one the daemon reached, or
    /// one that reached it: one on `daemons.accepted`.
    Daemon,
}

/// The daemon's side of every handshake — with a client, a provider or
/// another daemon alike, accepted or dialled. The rules, the same for
/// every connection:
///
/// 1. **Presenting.** The daemon presents, at an issuer, the token of
///    the subject its `identities` names there — see [`Own`]. The
///    dialler chooses the issuer, and nothing else.
/// 2. **Verifying.** The other end's token is verified against its
///    issuer's keys, trusting exactly the issuers `identities` names.
/// 3. **The two lists.** The daemon has exactly two:
///    `providers.accepted` and `daemons.accepted`. Whoever answers a
///    dial, and whoever dials in, must be on the list of the kind the
///    connection is for — see [`Expected`] — and may be any identity
///    on it. A socket the daemon accepted may also be an account, a
///    client, which is on no list.
/// 4. **The lock.** An identity on a list is held for as long as its
///    connection is open, in either direction: while it is held, it
///    is not a valid authentication, and a second connection under it
///    — dialled or dialling in — is refused. Accounts are never held,
///    and neither is the daemon's own identity. A handshake that fails
///    after taking an identity gives it back; the connection gives it
///    back when it ends, with [`release`](Self::release).
/// 5. **Refusing.** Anyone else is refused inside the handshake, which
///    closes the connection with no frame. A daemon with no account
///    and empty lists admits nothing at all.
///
/// [`expecting`](Self::expecting) is the authenticator one handshake
/// is made with; the daemon itself is none, so that no handshake is
/// made without saying which connection it is for.
#[derive(Debug)]
pub struct DaemonAuthenticator {
    own: Own,
    providers: HashSet<Identity>,
    daemons: HashSet<Identity>,
    store: Store,
    held_providers: Mutex<HashSet<Identity>>,
    held_daemons: Mutex<HashSet<Identity>>,
}

impl DaemonAuthenticator {
    /// The daemon as `own` presents it, its two lists `providers` and
    /// `daemons`, admitting the accounts of `store`; nothing held.
    pub fn new(own: Own, providers: impl IntoIterator<Item = Identity>, daemons: impl IntoIterator<Item = Identity>, store: Store) -> DaemonAuthenticator {
        DaemonAuthenticator {
            own,
            providers: providers.into_iter().collect(),
            daemons: daemons.into_iter().collect(),
            store,
            held_providers: Mutex::new(HashSet::new()),
            held_daemons: Mutex::new(HashSet::new()),
        }
    }

    /// The authenticator for one handshake for `expected` — see rule
    /// 3.
    pub fn expecting(&self, expected: Expected) -> Judging<'_> {
        Judging { authenticator: self, expected }
    }

    /// What `identity` is to this daemon on an accepted socket, past
    /// its handshake: a provider on the list, else the account it is,
    /// else `None`.
    pub async fn admit(&self, identity: &Identity) -> Result<Option<Admitted>, store::Error> {
        if self.providers.contains(identity) {
            return Ok(Some(Admitted::Provider(identity.clone())));
        }
        self.account(identity).await
    }

    /// The account `identity` is, as a client, or `None`.
    pub async fn account(&self, identity: &Identity) -> Result<Option<Admitted>, store::Error> {
        let mut conn = self.store.acquire().await?;
        Ok(accounts::by_identity(&mut conn, identity, false).await?.map(|account| Admitted::Client(Who::Account(account.id))))
    }

    /// The connection holding `identity`, of the kind `expected` names,
    /// ended: the identity is let go — rule 4. An account was never
    /// held, and is nothing to let go.
    pub async fn release(&self, expected: Expected, identity: &Identity) {
        if let Some(held) = self.list(expected, identity) {
            held.lock().await.remove(identity);
        }
    }

    /// The lock of the list `identity` is on for a connection for
    /// `expected`, if it is on it: on an accepted socket, a provider's;
    /// for a dialled provider, the providers'; for another daemon, the
    /// daemons'.
    fn list(&self, expected: Expected, identity: &Identity) -> Option<&Mutex<HashSet<Identity>>> {
        match expected {
            Expected::Admitted | Expected::Provider if self.providers.contains(identity) => Some(&self.held_providers),
            Expected::Daemon if self.daemons.contains(identity) => Some(&self.held_daemons),
            Expected::Admitted | Expected::Provider | Expected::Daemon => None,
        }
    }

    /// Rules 3 and 4 for `peer`: `Ok` with it taken, when it is on the
    /// list and free, or an account on an accepted socket; else why
    /// not.
    async fn take(&self, expected: Expected, peer: &Peer) -> Result<(), String> {
        if let Some(held) = self.list(expected, peer) {
            return if held.lock().await.insert(peer.clone()) { Ok(()) } else { Err(format!("{peer} is held by another connection")) };
        }
        match expected {
            Expected::Admitted => match self.account(peer).await {
                Ok(Some(_)) => Ok(()),
                Ok(None) => Err(format!("{peer} is neither an accepted provider nor an account")),
                Err(error) => Err(error.to_string()),
            },
            Expected::Provider => Err(format!("{peer} is not on providers.accepted")),
            Expected::Daemon => Err(format!("{peer} is not on daemons.accepted")),
        }
    }
}

/// The daemon's authenticator for one handshake: its own tokens, and
/// the list the other end must be on.
#[derive(Debug, Clone, Copy)]
pub struct Judging<'a> {
    authenticator: &'a DaemonAuthenticator,
    expected: Expected,
}

impl Authenticator for Judging<'_> {
    fn present<'a>(&'a self, issuer: &'a str) -> BoxFuture<'a, Result<String, String>> {
        Box::pin(self.authenticator.own.present(issuer))
    }

    fn verify<'a>(&'a self, token: &'a str, _address: IpAddr) -> BoxFuture<'a, Result<Peer, String>> {
        Box::pin(async move {
            let peer = self.authenticator.own.verify(token).await?;
            self.authenticator.take(self.expected, &peer).await?;
            Ok(peer)
        })
    }

    fn abandon<'a>(&'a self, peer: &'a Peer) -> BoxFuture<'a, ()> {
        Box::pin(self.authenticator.release(self.expected, peer))
    }
}
