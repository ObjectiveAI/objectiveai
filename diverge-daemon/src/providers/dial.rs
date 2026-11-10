//! A provider the daemon dials, dialled for the daemon's life.

use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;
use std::time::Duration;

use diverge_sdk::shared::listable::Listable;
use diverge_sdk::wire::connection::Connection;
use diverge_sdk::wire::handshake::{self, Handshook};
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

use diverge_sdk::shared::Identity;

use super::{attach, url};
use crate::containers::lifecycle::End;
use crate::daemon::Daemon;
use crate::judge::{Expected, Hold};

/// How long after a connection ends, or a dial fails, the next dial
/// is made: long enough that a provider that is down is not
/// hammered, short enough that one that is back is held soon.
const PAUSE: Duration = Duration::from_secs(5);

/// Dial the provider `identity` for as long as dial `number` is its
/// current one and its record enabled and outgoing.
///
/// Each round reads the record's address, connects to its [`url`],
/// makes the [`handshake`](diverge_sdk::wire::handshake) as the
/// connector — the daemon's own token presented, the provider's
/// verified against its own issuer, and required to be exactly
/// `identity`, which no other connection holds and which it then holds
/// — sends the listable flag the record says, and [`attach`]es: a
/// provider that answers its info is held and served until the
/// connection ends, or the daemon closes it. A handshake that fails is
/// a round not attached, the socket dropped. Then the pause, and again.
/// A round that finds the dial no longer current, or the record no
/// longer enabled and outgoing, ends the task, which forgets the dial
/// when it is still the current one; the daemon's stop ends it too, by
/// the abort handle [`Live`](crate::daemon::Live) keeps.
pub async fn dial(daemon: Arc<Daemon>, identity: Identity, number: u64) {
    rounds(&daemon, &identity, number).await;
    daemon.live.dial_ended(&identity, number).await;
}

/// The rounds of [`dial`], until one finds it should end.
async fn rounds(daemon: &Arc<Daemon>, identity: &Identity, number: u64) {
    loop {
        if !daemon.live.current_dial(identity, number).await {
            return;
        }
        let Some(address) = daemon.providers.dials(identity) else {
            return;
        };
        if let Ok((socket, _)) = connect_async(url(&address)).await {
            let peer = peer_ip(&socket);
            let expecting = daemon.authenticator.dialling(identity);
            let listable = if daemon.providers.listable(identity) { daemon.authenticator.listed() } else { Listable::Hidden };
            let made = handshake::connect(Connection::Outgoing(socket), listable, &expecting, peer).await;
            if let Ok(Handshook { connection, peer, .. }) = made {
                // A dial that answers holds the identity: the hold goes
                // with the connection, however it ends.
                let hold = Hold::new(&daemon.authenticator, Expected::Provider, peer.clone());
                if let Ok(attached) = attach(connection, peer, daemon, hold).await {
                    attached.serve().await;
                }
            }
        }
        tokio::time::sleep(PAUSE).await;
    }
}

/// Start a dial of `identity`, its current one from now: an earlier
/// task for it ends at its next turn.
pub async fn start_dial(daemon: &Arc<Daemon>, identity: Identity) {
    let number = daemon.live.begin_dial(identity.clone()).await;
    let task = tokio::spawn(dial(Arc::clone(daemon), identity.clone(), number));
    daemon.live.dial_task(identity, number, task.abort_handle()).await;
}

/// Start a dial for every enabled, outgoing record: what the daemon
/// does once at start.
pub async fn dial_all(daemon: &Arc<Daemon>) {
    for record in daemon.providers.all().iter() {
        if record.enabled && record.direction.dialled().is_some() {
            start_dial(daemon, record.identity.clone()).await;
        }
    }
}

/// A change to `identity`'s record, committed and read into the
/// [`ProviderBook`](super::ProviderBook), taking effect: the one place
/// it does. The rules:
///
/// 1. **A close.** When `end` is given, every run on the provider is
///    told it is ending so — its log's `undeployed` carries the reason —
///    and the connection to it, whichever end dialled, is closed: its
///    runs go down as on the socket ending.
/// 2. **The dial.** When the record is enabled and outgoing, it is
///    dialled: afresh after a close, or when no dial is current. When it
///    is not, its dial ends.
pub async fn reconcile(daemon: &Arc<Daemon>, identity: &Identity, end: Option<End>) {
    if let Some(end) = end {
        for run in daemon.live.agent_runs().await {
            if &run.provider == identity {
                run.end_as(end).await;
            }
        }
        for run in daemon.live.tool_runs().await {
            if &run.provider == identity {
                run.end_as(end).await;
            }
        }
        daemon.live.close_provider(identity).await;
    }
    if daemon.providers.dials(identity).is_some() {
        if end.is_some() || !daemon.live.dialling(identity).await {
            start_dial(daemon, identity.clone()).await;
        }
    } else {
        daemon.live.end_dial(identity).await;
    }
}

/// The peer's address, off the TCP stream under the WebSocket. With
/// no TLS in this crate the stream is always the plain one; anything
/// else, and a stream that will not say, is the unspecified address.
fn peer_ip(socket: &WebSocketStream<MaybeTlsStream<TcpStream>>) -> IpAddr {
    match socket.get_ref() {
        MaybeTlsStream::Plain(stream) => stream
            .peer_addr()
            .map(|address| address.ip())
            .unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED)),
        _ => IpAddr::V4(Ipv4Addr::UNSPECIFIED),
    }
}
