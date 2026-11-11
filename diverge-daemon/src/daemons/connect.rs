//! The one connection to a remote daemon: held, or opened now.

use std::sync::Arc;

use diverge_sdk::daemon::endpoints::daemons::DaemonId;
use diverge_sdk::provider::endpoints::daemons::connect::client::{execute as daemons_connect, request as daemons_request};
use rand::seq::IndexedRandom as _;
use tokio::sync::watch;

use super::{Fail, Peer};
use crate::daemon::Daemon;
use crate::store::daemons::Record;

/// The connection the record `record` names: the one held, while it has
/// not ended; else one opened now. The rules:
///
/// 1. **One per record.** A record's connection is kept by its ID and
///    reused by every relay and every connected tool naming it, so each
///    account of ours on a remote keeps its own.
/// 2. **Found by the listings.** Listings is touched — held, its listings
///    opened where they are not and waited on until whole — and the
///    routes to the record's remote that its
///    [`Routes`](diverge_sdk::daemon::endpoints::daemons::Routes) let
///    through are its candidates: none is [`Fail::NoProvider`], since a
///    remote is reached only while it is announced on a connected
///    provider by a route its record allows, and no route the filters
///    refuse is ever a fallback.
/// 3. **One at random.** One candidate is chosen at random — what a
///    measure of bandwidth or latency is to replace, see issue #315 —
///    and its provider's `daemons::connect` asked for the remote; its
///    refusal is [`Fail::Refused`].
/// 4. **As our account.** The handshake presents the record's identity
///    and admits only the record's remote — see
///    [`toward`](crate::judge::DaemonAuthenticator::toward).
/// 5. **Kept until it ends**, and a watcher forgetting it then — or
///    until the daemon forgets it first, its record deleted:
///    the watcher then lets it go, and it closes once nothing else of
///    the daemon's uses it.
/// 6. **One at a time.** Two connects naming one record are made one
///    after the other, the second finding the first's connection by
///    rule 1: a record never opens two.
pub async fn connect(daemon: &Arc<Daemon>, record: &Record) -> Result<Arc<Peer>, Fail> {
    // Rule 6.
    let _connecting = Connecting::hold(daemon, record.id).await;
    // Rule 1.
    if let Some(peer) = daemon.live.daemon_peer(record.id).await {
        if !peer.connected.is_ended() {
            return Ok(peer);
        }
        daemon.live.remove_peer(&peer).await;
    }
    // Rule 2.
    daemon.live.listings.touch(daemon).await;
    let candidates: Vec<_> = daemon.live.listings.routes(&record.daemon).await.into_iter().filter(|via| record.routes.admits(via)).collect();
    // Rule 3.
    let Some(provider) = candidates.choose(&mut rand::rng()).map(|via| via.provider.clone()) else {
        return Err(Fail::NoProvider);
    };
    let Some(handle) = daemon.live.provider(&provider).await else {
        return Err(Fail::NoProvider);
    };
    let request = daemons_request::Frame { daemon: record.daemon.clone() };
    // Rule 4.
    let authenticator = daemon.authenticator.toward(record.identity.clone(), record.daemon.clone());
    let connected = daemons_connect::execute(&handle, &request, &authenticator)
        .await
        .map_err(|error| Fail::Refused(error.to_string()))?;
    // Rule 5.
    let peer = Arc::new(Peer { daemon: record.id, provider, connected, closer: watch::channel(false).0 });
    daemon.live.insert_daemon(Arc::clone(&peer)).await;
    tokio::spawn(watch(Arc::clone(daemon), Arc::clone(&peer)));
    Ok(peer)
}

/// Forget the peer when its connection ends, so the next connect opens
/// anew; or let it go when the daemon forgot it first (rule 5).
async fn watch(daemon: Arc<Daemon>, peer: Arc<Peer>) {
    let closing = peer.closer.subscribe();
    tokio::select! {
        () = peer.connected.ended() => {}
        () = closed(closing) => {}
    }
    daemon.live.remove_peer(&peer).await;
}

/// Resolves once the peer's closer says so, or is gone.
async fn closed(mut closing: watch::Receiver<bool>) {
    while !*closing.borrow_and_update() {
        if closing.changed().await.is_err() {
            return;
        }
    }
}

/// The connect lock of one record, held for the length of its connect
/// (rule 6): let go however the connect ends, and its entry then
/// forgotten once nobody else holds it. A drop cannot wait on the live
/// map's lock, so that is a task of its own.
struct Connecting {
    /// The daemon whose map the entry is in.
    daemon: Arc<Daemon>,
    /// The record's ID.
    id: DaemonId,
    /// The lock, held; taken at the drop, so the entry is unshared
    /// before it is looked at.
    held: Option<tokio::sync::OwnedMutexGuard<()>>,
}

impl Connecting {
    /// The connect lock of the record `id`, waited for and held.
    async fn hold(daemon: &Arc<Daemon>, id: DaemonId) -> Connecting {
        let lock = daemon.live.connecting(id).await;
        let held = lock.lock_owned().await;
        Connecting { daemon: Arc::clone(daemon), id, held: Some(held) }
    }
}

impl Drop for Connecting {
    fn drop(&mut self) {
        drop(self.held.take());
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            let (daemon, id) = (Arc::clone(&self.daemon), self.id);
            runtime.spawn(async move { daemon.live.connected_to(id).await });
        }
    }
}
