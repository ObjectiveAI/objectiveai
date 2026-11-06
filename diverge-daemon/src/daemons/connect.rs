//! The one connection to a remote daemon: held, or opened now.

use std::sync::Arc;

use diverge_sdk::provider::endpoints::daemons::connect::client::{execute as daemons_connect, request as daemons_request};
use rand::seq::IndexedRandom as _;

use super::{Fail, Peer};
use crate::daemon::Daemon;
use crate::store::daemons::Record;

/// The connection the record `record` names: the one held, while it has
/// not ended; else one opened now. The rules:
///
/// 1. **One per record.** A record's connection is kept by its name and
///    reused by every relay and every connected tool naming it, so each
///    account of ours on a remote keeps its own.
/// 2. **Found by discovery.** Discovery is touched — held, its listings
///    opened where they are not and waited on until whole — and the
///    routes to the record's remote that its
///    [`Routes`](diverge_sdk::daemon::endpoints::daemons::Routes) let
///    through are its candidates: none is [`Fail::NoProvider`], since a
///    remote is reached only while it is discoverable on a connected
///    provider by a route its record allows, and no route the filters
///    refuse is ever a fallback.
/// 3. **One at random.** One candidate is chosen at random — what a
///    measure of bandwidth or latency is to replace, see issue #315 —
///    and its provider's `daemons::connect` asked for the remote; its
///    refusal is [`Fail::Refused`].
/// 4. **As our account.** The handshake presents the record's identity
///    and admits only the record's remote — see
///    [`toward`](crate::judge::DaemonAuthenticator::toward).
/// 5. **Kept until it ends**, and a watcher forgetting it then.
pub async fn connect(daemon: &Arc<Daemon>, record: &Record) -> Result<Arc<Peer>, Fail> {
    // Rule 1.
    if let Some(peer) = daemon.live.daemon_peer(&record.name).await {
        if !peer.connected.is_ended() {
            return Ok(peer);
        }
        daemon.live.remove_peer(&peer).await;
    }
    // Rule 2.
    daemon.live.discovery.touch(daemon).await;
    let candidates: Vec<_> = daemon.live.discovery.routes(&record.daemon).await.into_iter().filter(|via| record.routes.admits(via)).collect();
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
    let connected = daemons_connect::execute(&handle, &request, &record.daemon.issuer, &authenticator)
        .await
        .map_err(|error| Fail::Refused(error.to_string()))?;
    // Rule 5.
    let peer = Arc::new(Peer { daemon: record.name.clone(), provider, connected });
    daemon.live.insert_daemon(Arc::clone(&peer)).await;
    tokio::spawn(watch(Arc::clone(daemon), Arc::clone(&peer)));
    Ok(peer)
}

/// Forget the peer when its connection ends, so the next connect opens
/// anew.
async fn watch(daemon: Arc<Daemon>, peer: Arc<Peer>) {
    peer.connected.ended().await;
    daemon.live.remove_peer(&peer).await;
}
