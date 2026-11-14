//! The handshake: a verified identity becomes an account, a provider,
//! a remote daemon, or nothing.

use std::collections::HashSet;
use std::net::IpAddr;
use std::sync::Arc;

use diverge_sdk::identities::{Identities, Own};
use diverge_sdk::shared::Identity;
use diverge_sdk::wire::handshake::{Authenticator, Judged, Peer, Presented};
use futures_util::future::BoxFuture;
use tokio::sync::{Mutex, watch};

use super::Who;
use crate::providers::ProviderBook;
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
    /// A socket the daemon accepted: a provider with an enabled,
    /// incoming record admitting the source address, an administrator
    /// on `admins`, or an account.
    Admitted,
    /// A provider the daemon dialled: the one its enabled, outgoing
    /// record names.
    Provider,
    /// A daemon that reached this one through a provider: an account
    /// of this daemon's, and on no list.
    Relayed,
}

/// The daemon's side of every handshake — with a client, a provider or
/// another daemon alike, accepted or dialled. The rules, the same for
/// every connection:
///
/// 1. **Presenting.** The daemon presents the token of its one
///    `identity` — see [`Own`] — on every socket it accepts, to every
///    daemon that reaches it, and to every provider it dials, whatever
///    issuer the other end's token is from. Only [`Toward`] presents
///    another: the account of ours a remote daemon's record names. The
///    token is all a handshake carries: what the daemon announces
///    itself with on a provider is a scope it opens after — see
///    [`sync`](crate::providers::sync).
/// 2. **Verifying.** There is no list of issuers. The other end's
///    token is read for whom it claims to be, unsigned; that identity
///    is judged by rules 3 and 5; and only one that passes has its
///    token verified, against its own issuer's keys. A token claiming
///    anyone else is refused with nothing fetched.
/// 3. **The list.** The daemon has exactly one: its providers' records
///    — see [`ProviderBook`]. A provider that dials in must have an
///    enabled, incoming record whose ips contain the socket's
///    source; a provider that answers a dial must be exactly the one
///    the dial's enabled, outgoing record names — see
///    [`dialling`](Self::dialling). A daemon this daemon reaches is
///    judged by its record instead — see [`toward`](Self::toward) — and
///    one that reaches this daemon is on no list: it must be an account
///    of this daemon's. A socket the daemon accepted may also be an
///    account, a client, which is on no list.
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
    providers: Arc<ProviderBook>,
    admins: HashSet<Identity>,
    store: Store,
    held_providers: Mutex<HashSet<Identity>>,
    held_accounts: Mutex<HashSet<Identity>>,
}

impl DaemonAuthenticator {
    /// The daemon as `own` presents it, its providers' records
    /// `providers`, its `admins`, admitting the accounts of `store`;
    /// nothing held.
    pub fn new(own: Own, providers: Arc<ProviderBook>, admins: impl IntoIterator<Item = Identity>, store: Store) -> DaemonAuthenticator {
        DaemonAuthenticator {
            own,
            providers,
            admins: admins.into_iter().collect(),
            store,
            held_providers: Mutex::new(HashSet::new()),
            held_accounts: Mutex::new(HashSet::new()),
        }
    }

    /// The authenticator for one handshake for `expected` — see rule
    /// 3.
    pub fn expecting(&self, expected: Expected) -> Judging<'_> {
        Judging { authenticator: self, expected, dialled: None, took: None }
    }

    /// The authenticator for one handshake on a socket the daemon
    /// accepted, as [`expecting`](Self::expecting) `Admitted`, which
    /// turns `took` `true` when it holds the identity as a provider's:
    /// whose hold the connection then has to give back, by a
    /// [`Hold`].
    pub fn admitting<'a>(&'a self, took: &'a watch::Sender<bool>) -> Judging<'a> {
        Judging { authenticator: self, expected: Expected::Admitted, dialled: None, took: Some(took) }
    }

    /// The authenticator for one dial of the provider `identity`: the
    /// answer must be it, and its record enabled and outgoing — rule 3.
    /// A dial that answers holds the identity always.
    pub fn dialling<'a>(&'a self, identity: &'a Identity) -> Judging<'a> {
        Judging { authenticator: self, expected: Expected::Provider, dialled: Some(identity), took: None }
    }

    /// The authenticator for reaching the remote daemon `remote` as our
    /// account `ours` there, as a record of it names the two — see
    /// [`Toward`].
    pub fn toward(&self, ours: Identity, remote: Identity) -> Toward<'_> {
        Toward { authenticator: self, ours, remote }
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
        if self.providers.accepts(identity) {
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

    /// The identity this daemon is, if its configuration names one:
    /// whom it answers every handshake as, and whom it dials every
    /// provider as.
    pub fn own(&self) -> Option<&Identity> {
        self.own.identity()
    }

    /// Every identity on the host, as the daemon opened them at its
    /// start: what a listing of them reads.
    pub fn identities(&self) -> &Identities {
        self.own.identities()
    }

    /// The connection holding `identity`, of the kind `expected` names,
    /// ended: the identity is let go — rule 4. A client socket's
    /// account was never held, and is nothing to let go.
    pub async fn release(&self, expected: Expected, identity: &Identity) {
        match expected {
            // A provider's record may be gone by now: its hold goes
            // whether or not.
            Expected::Admitted | Expected::Provider => {
                self.held_providers.lock().await.remove(identity);
            }
            Expected::Relayed => {
                self.held_accounts.lock().await.remove(identity);
            }
        }
    }

    /// The lock `identity` is held under for a connection for
    /// `expected`, if it is held at all: on an accepted socket, or a
    /// dialled provider, the providers', for one with a record; for a
    /// daemon reaching this one, the accounts'.
    fn list(&self, expected: Expected, identity: &Identity) -> Option<&Mutex<HashSet<Identity>>> {
        match expected {
            Expected::Admitted | Expected::Provider if self.providers.accepts(identity) => Some(&self.held_providers),
            Expected::Relayed => Some(&self.held_accounts),
            Expected::Admitted | Expected::Provider => None,
        }
    }

    /// Rules 3 and 5 for `peer`, connected from `address`, nothing
    /// taken: `Ok` when it is a provider its record admits in this
    /// direction — the one `dialled` names, for a dial — or an account
    /// a daemon reaches this one as, or an administrator or an account
    /// on an accepted socket; else why not. Asked of whom a token
    /// claims to be, before the token is verified — rule 2.
    async fn admissible(&self, expected: Expected, peer: &Peer, address: IpAddr, dialled: Option<&Identity>) -> Result<(), String> {
        match expected {
            Expected::Relayed => match self.account(peer).await {
                Ok(Some(_)) => Ok(()),
                Ok(None) => Err(format!("{peer} is not an account")),
                Err(error) => Err(error.to_string()),
            },
            Expected::Provider => {
                if dialled.is_some_and(|dialled| dialled != peer) {
                    return Err(format!("{peer} answered a dial of another provider"));
                }
                if self.providers.dials(peer).is_none() {
                    return Err(format!("{peer} is no enabled, outgoing provider"));
                }
                if !self.providers.accepts(peer) {
                    return Err(format!("{peer} is no provider of this daemon's"));
                }
                Ok(())
            }
            Expected::Admitted if self.providers.accepts(peer) => {
                if self.providers.admits(peer, address) { Ok(()) } else { Err(format!("{peer} may not dial in from {address}")) }
            }
            Expected::Admitted if self.admins.contains(peer) => Ok(()),
            Expected::Admitted => match self.account(peer).await {
                Ok(Some(_)) => Ok(()),
                Ok(None) => Err(format!("{peer} is neither a provider, an administrator nor an account")),
                Err(error) => Err(error.to_string()),
            },
        }
    }

    /// Rule 4 for `peer`, verified and [`admissible`](Self::admissible):
    /// `Ok(true)` when it was taken on a list, held until it is
    /// [`release`](Self::release)d; `Ok(false)` when it is on no list;
    /// why not, when another connection holds it.
    async fn take(&self, expected: Expected, peer: &Peer) -> Result<bool, String> {
        match self.list(expected, peer) {
            Some(held) => {
                if held.lock().await.insert(peer.clone()) { Ok(true) } else { Err(format!("{peer} is held by another connection")) }
            }
            None => Ok(false),
        }
    }
}

/// A connection's hold on a provider's identity, from its handshake to
/// its end: given back once — [`release`](Hold::release) on the way
/// out, or the drop however else the connection ends: an error, a
/// panic, its task aborted. A drop cannot wait on the list's lock, so
/// the giving back is a task of its own.
pub struct Hold {
    /// Whose list it is held on.
    authenticator: Arc<DaemonAuthenticator>,
    /// The connection's kind: which list.
    expected: Expected,
    /// The identity held, until it is given back.
    identity: Option<Identity>,
}

impl Hold {
    /// The hold the handshake for `expected` took on `identity`, to be
    /// given back once.
    pub fn new(authenticator: &Arc<DaemonAuthenticator>, expected: Expected, identity: Identity) -> Hold {
        Hold { authenticator: Arc::clone(authenticator), expected, identity: Some(identity) }
    }

    /// The connection is over: the identity given back now, waited for.
    pub async fn release(mut self) {
        if let Some(identity) = &self.identity {
            self.authenticator.release(self.expected, identity).await;
        }
        self.identity = None;
    }
}

impl Drop for Hold {
    fn drop(&mut self) {
        let Some(identity) = self.identity.take() else {
            return;
        };
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            let (authenticator, expected) = (Arc::clone(&self.authenticator), self.expected);
            runtime.spawn(async move { authenticator.release(expected, &identity).await });
        }
    }
}

/// The daemon's authenticator for one handshake: its own tokens, and
/// the list the other end must be on.
#[derive(Debug, Clone, Copy)]
pub struct Judging<'a> {
    authenticator: &'a DaemonAuthenticator,
    expected: Expected,
    /// The provider a dial expects to answer, for a dial.
    dialled: Option<&'a Identity>,
    /// Turned `true` when an accepted socket's identity is held on a
    /// list: see [`admitting`](DaemonAuthenticator::admitting).
    took: Option<&'a watch::Sender<bool>>,
}

impl Authenticator for Judging<'_> {
    fn present<'a>(&'a self) -> BoxFuture<'a, Result<Presented, String>> {
        Box::pin(self.authenticator.own.present())
    }

    fn verify<'a>(&'a self, token: &'a str, address: IpAddr) -> BoxFuture<'a, Result<Judged, String>> {
        Box::pin(async move {
            // Rule 2: whom it claims to be is judged first, and only
            // then is its issuer trusted and its keys fetched.
            let claimed = self.authenticator.own.claimed(token)?;
            self.authenticator.admissible(self.expected, &claimed, address, self.dialled).await?;
            // The identity, and the key its token names: the handshake
            // holds the other end to the key.
            let judged = self.authenticator.own.verify_from(token, &claimed.issuer).await?;
            let peer = &judged.peer;
            if *peer != claimed {
                return Err(format!("{peer} is not {claimed}, whom the token claimed"));
            }
            let held = self.authenticator.take(self.expected, peer).await?;
            if held && let Some(took) = self.took {
                took.send_replace(true);
            }
            Ok(judged)
        })
    }

    fn abandon<'a>(&'a self, peer: &'a Peer) -> BoxFuture<'a, ()> {
        Box::pin(self.authenticator.release(self.expected, peer))
    }
}

/// The daemon's authenticator for reaching one remote daemon as one
/// account of ours there, as a record of it names the two. The rules:
///
/// 1. **Presenting.** The ID token of `ours`, one of the host's
///    identities, whatever the daemon's own `identity` is, and that
///    identity's key on this host.
/// 2. **Verifying.** The answer's token is verified against the remote's
///    issuer's keys, that issuer trusted for this answer alone, and
///    must name a key, which the handshake holds the remote to.
/// 3. **The one answer.** The answer must be `remote`, and nothing else:
///    a record names the daemon it reaches.
/// 4. **Nothing held.** Several records may reach the same remote, each
///    as another account of ours, at once: the record's one connection
///    is the daemon's live peer of that name, not a lock here.
#[derive(Debug, Clone)]
pub struct Toward<'a> {
    authenticator: &'a DaemonAuthenticator,
    ours: Identity,
    remote: Identity,
}

impl Authenticator for Toward<'_> {
    fn present<'a>(&'a self) -> BoxFuture<'a, Result<Presented, String>> {
        Box::pin(async move {
            let name = diverge_sdk::identities::Name { issuer: self.ours.issuer.clone(), subject: self.ours.subject.clone() };
            self.authenticator.own.present_as(&name).await
        })
    }

    fn verify<'a>(&'a self, token: &'a str, _address: IpAddr) -> BoxFuture<'a, Result<Judged, String>> {
        Box::pin(async move {
            let judged = self.authenticator.own.verify_from(token, &self.remote.issuer).await?;
            if judged.peer != self.remote {
                return Err(format!("{} answered where {} was reached", judged.peer, self.remote));
            }
            Ok(judged)
        })
    }

    fn abandon<'a>(&'a self, _peer: &'a Peer) -> BoxFuture<'a, ()> {
        Box::pin(async {})
    }
}
