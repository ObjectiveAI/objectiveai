//! An outgoing provider, dialled for its life on record.

use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;
use std::time::Duration;

use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use diverge_sdk::daemon::endpoints::providers::outgoing::Mode;
use diverge_sdk::wire::client::authorization::{Auth, Authorization};
use diverge_sdk::wire::client::authorize::authorize;
use diverge_sdk::wire::client::authorized::Authorized;
use diverge_sdk::wire::connection::Connection;
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

use super::{Refusing, attach, url};
use crate::daemon::Daemon;
use crate::store::{self, providers_outgoing};

/// How long after a connection ends, or a dial fails, the next dial
/// is made: long enough that a provider that is down is not
/// hammered, short enough that one that is back is held soon.
const PAUSE: Duration = Duration::from_secs(5);

/// Dial the outgoing provider at `address` for its life on record.
///
/// Each round reads the provider's record — so an edit's new mode is
/// what the next dial presents, and a provider forgotten ends the
/// task — connects to its [`url`], presents the mode's authorization
/// as the connection's first frame, and [`attach`]es: a provider that
/// answers its version is held and served until the connection ends,
/// with the record's `last_connected` set as it opens and as it
/// closes. Then the pause, and again. Ended only by the abort handle
/// [`Live`](crate::daemon::Live) keeps for it, which drops whatever
/// connection it holds.
pub async fn dial(daemon: Arc<Daemon>, address: String) {
    loop {
        let Ok(Some(provider)) = on_record(&daemon, &address).await else {
            return;
        };
        if let Ok((socket, _)) = connect_async(url(&address)).await {
            let peer = peer_ip(&socket);
            let Mode::Unbrokered { authorization } = provider.mode;
            let presented = Authorization::<Refusing>::Outgoing {
                auth: Auth::Unbrokered(authorization),
                provider_identity: address.clone(),
            };
            if let Ok(Authorized { connection, .. }) = authorize(Connection::Outgoing(socket), presented, peer).await
                && let Ok(attached) = attach(
                    connection,
                    Identity::Outgoing {
                        address: address.clone(),
                    },
                    &daemon,
                )
                .await
            {
                let _ = connected_now(&daemon, provider.id).await;
                attached.serve().await;
                let _ = connected_now(&daemon, provider.id).await;
            }
        }
        tokio::time::sleep(PAUSE).await;
    }
}

/// Start a dial for every outgoing provider on record: what the
/// daemon does once at start.
pub async fn dial_all(daemon: &Arc<Daemon>) -> Result<(), store::Error> {
    let providers = {
        let mut conn = daemon.store.acquire().await?;
        providers_outgoing::all(&mut conn).await?
    };
    for provider in providers {
        start(daemon, provider.address).await;
    }
    Ok(())
}

/// Start the dial task for `address`, and keep its abort handle.
pub async fn start(daemon: &Arc<Daemon>, address: String) {
    let task = tokio::spawn(dial(Arc::clone(daemon), address.clone()));
    daemon.live.start_dial(address, task.abort_handle()).await;
}

/// The provider's record now, if it is still on record.
async fn on_record(daemon: &Daemon, address: &str) -> Result<Option<providers_outgoing::Outgoing>, store::Error> {
    let mut conn = daemon.store.acquire().await?;
    providers_outgoing::by_address(&mut conn, address, false).await
}

/// Record that the connection opened or closed now.
async fn connected_now(daemon: &Daemon, id: store::OutgoingId) -> Result<(), store::Error> {
    let mut conn = daemon.store.acquire().await?;
    providers_outgoing::set_last_connected(&mut conn, id).await
}

/// The peer's address, off the TCP stream under the WebSocket. With
/// no TLS in this crate the stream is always the plain one; anything
/// else, and a stream that will not say, is the unspecified address.
/// The handshake takes it, and on a connection this end dialled
/// nothing reads it.
fn peer_ip(socket: &WebSocketStream<MaybeTlsStream<TcpStream>>) -> IpAddr {
    match socket.get_ref() {
        MaybeTlsStream::Plain(stream) => stream
            .peer_addr()
            .map(|address| address.ip())
            .unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED)),
        _ => IpAddr::V4(Ipv4Addr::UNSPECIFIED),
    }
}
