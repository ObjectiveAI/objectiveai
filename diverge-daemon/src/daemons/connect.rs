//! The one connection to a daemon as one of its accounts: held, or
//! opened now.

use std::sync::Arc;

use diverge_sdk::daemon::endpoints::providers::daemons::Link;
use diverge_sdk::daemon::endpoints::providers::outgoing::Mode;
use diverge_sdk::provider::endpoints::daemons::connect::client::{execute as daemons_connect, request as daemons_request};
use diverge_sdk::shared::daemons;
use rand::seq::SliceRandom as _;

use super::{Fail, Peer};
use crate::daemon::Daemon;
use crate::store::providers_daemons::Record;

/// The connection to the daemon the record names, as the account of
/// `account`: the one held, while it has not ended; else one opened
/// now through the record's links whose providers are connected, in
/// random order — so that a daemon reachable through several
/// providers is reached through any of them — presenting the
/// account's credential, the first that answers kept until it ends,
/// and a watcher forgetting it then. No account of the name is
/// [`Fail::NoAccount`]; a key the account's mode names that the
/// keyring does not hold is [`Fail::NoKey`]; a link whose provider is not connected is
/// skipped; none connected is [`Fail::NoProvider`]; every one tried
/// and refused is [`Fail::Refused`] with the last refusal.
pub async fn connect(daemon: &Arc<Daemon>, record: &Record, account: &str) -> Result<Arc<Peer>, Fail> {
    if let Some(peer) = daemon.live.daemon_peer(&record.name, account).await {
        if !peer.connected.is_ended() {
            return Ok(peer);
        }
        daemon.live.remove_peer(&peer).await;
    }
    let Some(held) = record.account(account) else {
        return Err(Fail::NoAccount);
    };
    let Mode::Unbrokered { key } = &held.mode;
    let key = match daemon.keyring.get(key).await {
        Ok(Some(key)) => key,
        Ok(None) => return Err(Fail::NoKey(key.clone())),
        Err(error) => return Err(Fail::Keyring(error.to_string())),
    };
    let mut links: Vec<&Link> = record.links.iter().collect();
    links.shuffle(&mut rand::rng());
    let mut last = None;
    for link in links {
        let Some(handle) = daemon.live.provider(&link.provider).await else {
            continue;
        };
        let request = daemons_request::Frame {
            daemon: link.identity.clone(),
            mode: daemons::Mode::Unbrokered {
                credential: key.expose().to_string(),
            },
        };
        match daemons_connect::execute(&handle, &request).await {
            Ok(connected) => {
                let peer = Arc::new(Peer {
                    daemon: record.name.clone(),
                    account: account.to_string(),
                    link: link.clone(),
                    connected,
                });
                daemon.live.insert_daemon(Arc::clone(&peer)).await;
                tokio::spawn(watch(Arc::clone(daemon), Arc::clone(&peer)));
                return Ok(peer);
            }
            Err(error) => last = Some(format!("{error:?}")),
        }
    }
    Err(match last {
        Some(error) => Fail::Refused(error),
        None => Fail::NoProvider,
    })
}

/// Forget the peer when its connection ends, so the next connect
/// opens anew.
async fn watch(daemon: Arc<Daemon>, peer: Arc<Peer>) {
    peer.connected.ended().await;
    daemon.live.remove_peer(&peer).await;
}
