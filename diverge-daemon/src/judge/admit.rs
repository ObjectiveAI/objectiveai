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
    /// A client: an administrator, or the account the identity is.
    Client(Who),
    /// A provider the daemon works with.
    Provider(Identity),
}

/// Which connection a handshake is for: what decides the list the
/// other end must be on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Expected {
    /// A socket the daemon accepted: a provider on `providers.accepted`,
    /// an administrator on `admins`, or an account.
    Admitted,
    /// A provider the daemon dialled: one on `providers.accepted`.
    Provider,
    /// A daemon this daemon reached through a provider: one on
    /// `daemons.accepted`.
    Daemon,
    /// A daemon that reached this one through a provider: an account
    /// of this daemon's, and on no list.
    Relayed,
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
///    `providers.accepted`, for every provider, whichever end dialled,
///    and `daemons.accepted`, for the daemons this daemon reaches.
///    Whoever answers a dial, and a provider that dials in, must be on
///    the list of the kind the connection is for — see [`Expected`] —
///    and may be any identity on it. A daemon that reaches this one is
///    on no list: it must be an account of this daemon's. A socket the
///    daemon accepted may also be an account, a client, which is on no
///    list.
/// 4. **The lock.** An identity on a list is held for as long as its
///    connection is open, in either direction, and so is the account a
///    daemon reaches this one as: while it is held, it is not a valid
///    authentication, and a second connection under it is refused — an
///    account has at most one daemon connecting as it at a time. A
///    client socket's account is never held, and does not count, and
///    neither is the daemon's own identity. A handshake that fails
///    after taking an identity gives it back; the connection gives it
///    back when it ends, with [`release`](Self::release).
/// 5. **Administrators.** An identity `admins` names is admitted on an
///    accepted socket as a client, account or not, and holds every
///    grant there is; it is on no list, and is never held. What makes a
///    fresh daemon, with no account, reachable: its administrators make
///    the first accounts.
/// 6. **Refusing.** Anyone else is refused inside the handshake, which
///    closes the connection with no frame. A daemon with no account, no
///    administrator and empty lists admits nothing at all.
///
/// [`expecting`](Self::expecting) is the authenticator one handshake
/// is made with; the daemon itself is none, so that no handshake is
/// made without saying which connection it is for.
#[derive(Debug)]
pub struct DaemonAuthenticator {
    own: Own,
    providers: HashSet<Identity>,
    daemons: HashSet<Identity>,
    admins: HashSet<Identity>,
    store: Store,
    held_providers: Mutex<HashSet<Identity>>,
    held_daemons: Mutex<HashSet<Identity>>,
    held_accounts: Mutex<HashSet<Identity>>,
}

impl DaemonAuthenticator {
    /// The daemon as `own` presents it, its two lists `providers` and
    /// `daemons`, its `admins`, admitting the accounts of `store`;
    /// nothing held.
    pub fn new(
        own: Own,
        providers: impl IntoIterator<Item = Identity>,
        daemons: impl IntoIterator<Item = Identity>,
        admins: impl IntoIterator<Item = Identity>,
        store: Store,
    ) -> DaemonAuthenticator {
        DaemonAuthenticator {
            own,
            providers: providers.into_iter().collect(),
            daemons: daemons.into_iter().collect(),
            admins: admins.into_iter().collect(),
            store,
            held_providers: Mutex::new(HashSet::new()),
            held_daemons: Mutex::new(HashSet::new()),
            held_accounts: Mutex::new(HashSet::new()),
        }
    }

    /// The authenticator for one handshake for `expected` — see rule
    /// 3.
    pub fn expecting(&self, expected: Expected) -> Judging<'_> {
        Judging { authenticator: self, expected }
    }

    /// The administrators the configuration names.
    pub fn admins(&self) -> impl Iterator<Item = &Identity> {
        self.admins.iter()
    }

    /// Whether the configuration names `identity` an administrator.
    pub fn is_admin(&self, identity: &Identity) -> bool {
        self.admins.contains(identity)
    }

    /// What `identity` is to this daemon on an accepted socket, past
    /// its handshake: a provider on the list, else an administrator on
    /// `admins`, else the account it is, else `None`.
    pub async fn admit(&self, identity: &Identity) -> Result<Option<Admitted>, store::Error> {
        if self.providers.contains(identity) {
            return Ok(Some(Admitted::Provider(identity.clone())));
        }
        if self.admins.contains(identity) {
            return Ok(Some(Admitted::Client(Who::Admin(identity.clone()))));
        }
        self.account(identity).await
    }

    /// The account `identity` is, as a client, or `None`.
    pub async fn account(&self, identity: &Identity) -> Result<Option<Admitted>, store::Error> {
        let mut conn = self.store.acquire().await?;
        Ok(accounts::by_identity(&mut conn, identity, false).await?.map(|account| Admitted::Client(Who::Account(account.id))))
    }

    /// The connection holding `identity`, of the kind `expected` names,
    /// ended: the identity is let go — rule 4. A client socket's
    /// account was never held, and is nothing to let go.
    pub async fn release(&self, expected: Expected, identity: &Identity) {
        if let Some(held) = self.list(expected, identity) {
            held.lock().await.remove(identity);
        }
    }

    /// The lock `identity` is held under for a connection for
    /// `expected`, if it is held at all: on an accepted socket, or a
    /// dialled provider, the providers', for one on the list; for a
    /// daemon reached, the daemons', for one on the list; for a daemon
    /// reaching this one, the accounts'.
    fn list(&self, expected: Expected, identity: &Identity) -> Option<&Mutex<HashSet<Identity>>> {
        match expected {
            Expected::Admitted | Expected::Provider if self.providers.contains(identity) => Some(&self.held_providers),
            Expected::Daemon if self.daemons.contains(identity) => Some(&self.held_daemons),
            Expected::Relayed => Some(&self.held_accounts),
            Expected::Admitted | Expected::Provider | Expected::Daemon => None,
        }
    }

    /// Rules 3 and 4 for `peer`: `Ok` with it taken, when it is on the
    /// list and free, or an account a daemon reaches this one as and
    /// free, or an account on an accepted socket; else why not.
    async fn take(&self, expected: Expected, peer: &Peer) -> Result<(), String> {
        if expected == Expected::Relayed {
            match self.account(peer).await {
                Ok(Some(_)) => {}
                Ok(None) => return Err(format!("{peer} is not an account")),
                Err(error) => return Err(error.to_string()),
            }
        }
        if let Some(held) = self.list(expected, peer) {
            return if held.lock().await.insert(peer.clone()) { Ok(()) } else { Err(format!("{peer} is held by another connection")) };
        }
        match expected {
            Expected::Admitted if self.admins.contains(peer) => Ok(()),
            Expected::Admitted => match self.account(peer).await {
                Ok(Some(_)) => Ok(()),
                Ok(None) => Err(format!("{peer} is neither an accepted provider, an administrator nor an account")),
                Err(error) => Err(error.to_string()),
            },
            Expected::Provider => Err(format!("{peer} is not on providers.accepted")),
            Expected::Daemon => Err(format!("{peer} is not on daemons.accepted")),
            Expected::Relayed => Ok(()),
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
