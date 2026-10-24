//! A provider the daemon dials, dialled for the daemon's life.

use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;
use std::time::Duration;

use diverge_sdk::wire::connection::Connection;
use diverge_sdk::wire::handshake::{self, Handshook};
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

use super::{attach, url};
use crate::config::Connect;
use crate::daemon::Daemon;
use crate::judge::Expected;

/// How long after a connection ends, or a dial fails, the next dial
/// is made: long enough that a provider that is down is not
/// hammered, short enough that one that is back is held soon.
const PAUSE: Duration = Duration::from_secs(5);

/// Dial the provider `connect` names for the daemon's life.
///
/// Each round connects to its [`url`], makes the
/// [`handshake`](diverge_sdk::wire::handshake) as the connector — the
/// daemon's token from `connect.issuer`, the provider's from the same
/// issuer verified, and required to be one the configuration's
/// `providers.accepted` lists — and [`attach`]es under the identity the
/// provider answered as: a provider that answers its info is
/// held and served until the connection ends. A handshake that fails
/// — an identity not accepted among the ways — is a round not
/// attached, the socket dropped. Then the pause, and again. Ended only by the abort handle
/// [`Live`](crate::daemon::Live) keeps for it, which drops whatever
/// connection it holds.
pub async fn dial(daemon: Arc<Daemon>, connect: Connect) {
    loop {
        if let Ok((socket, _)) = connect_async(url(&connect.address)).await {
            let peer = peer_ip(&socket);
            let expecting = daemon.authenticator.expecting(Expected::Provider);
            if let Ok(Handshook { connection, peer }) = handshake::connect(Connection::Outgoing(socket), &connect.issuer, &expecting, peer).await {
                daemon.live.dialled(peer.clone(), connect.address.clone()).await;
                if let Ok(attached) = attach(connection, peer, &daemon).await {
                    attached.serve().await;
                }
            }
        }
        tokio::time::sleep(PAUSE).await;
    }
}

/// Start a dial for every provider the configuration's
/// `providers.connect` lists: what the daemon does once at start.
pub async fn dial_all(daemon: &Arc<Daemon>) {
    for connect in &daemon.providers.connect {
        let task = tokio::spawn(dial(Arc::clone(daemon), connect.clone()));
        daemon.live.start_dial(connect.address.clone(), task.abort_handle()).await;
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
