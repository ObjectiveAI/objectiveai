//! The one connection to a remote daemon: held, or opened now.

use std::sync::Arc;

use diverge_sdk::daemon::endpoints::providers::daemons::Link;
use diverge_sdk::provider::endpoints::daemons::connect::client::{execute as daemons_connect, request as daemons_request};
use rand::seq::SliceRandom as _;

use super::{Fail, Peer};
use crate::config::Remote;
use crate::daemon::Daemon;
use crate::judge::Expected;

/// The connection to `remote`: the one held, while it has not ended;
/// else one opened now through the remote's links whose providers are
/// connected, in random order — so that a daemon reachable through
/// several providers is reached through any of them — the remote named
/// by the link's `daemon` identity, the relayed connection's handshake
/// made from the remote's `issuer` and the answer required to be a
/// daemon on `daemons.accepted` that no connection holds, which it then
/// holds; the first that answers kept until
/// it ends, and a watcher forgetting it then. A link whose provider is
/// not connected is skipped; none connected is [`Fail::NoProvider`];
/// every one tried and refused is [`Fail::Refused`] with the last
/// refusal.
pub async fn connect(daemon: &Arc<Daemon>, remote: &Remote) -> Result<Arc<Peer>, Fail> {
    if let Some(peer) = daemon.live.daemon_peer(&remote.name).await {
        if !peer.connected.is_ended() {
            return Ok(peer);
        }
        daemon.live.remove_peer(&peer).await;
    }
    let mut links: Vec<&Link> = remote.links.iter().collect();
    links.shuffle(&mut rand::rng());
    let mut last = None;
    for link in links {
        let Some(handle) = daemon.live.provider(&link.provider).await else {
            continue;
        };
        let request = daemons_request::Frame { daemon: link.daemon.clone() };
        match daemons_connect::execute(&handle, &request, &remote.issuer, &daemon.authenticator.expecting(Expected::Daemon)).await {
            Ok(connected) => {
                let peer = Arc::new(Peer {
                    daemon: remote.name.clone(),
                    link: link.clone(),
                    connected,
                });
                daemon.live.insert_daemon(Arc::clone(&peer)).await;
                tokio::spawn(watch(Arc::clone(daemon), Arc::clone(&peer)));
                return Ok(peer);
            }
            Err(error) => last = Some(error.to_string()),
        }
    }
    Err(match last {
        Some(error) => Fail::Refused(error),
        None => Fail::NoProvider,
    })
}

/// Forget the peer when its connection ends, so the next connect
/// opens anew, and let go of the identity the handshake held.
async fn watch(daemon: Arc<Daemon>, peer: Arc<Peer>) {
    peer.connected.ended().await;
    daemon.live.remove_peer(&peer).await;
    daemon.authenticator.release(Expected::Daemon, &peer.connected.peer).await;
}
