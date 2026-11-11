//! One peer the provider dials, for the provider's life.

use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;

use diverge_sdk::wire::connection::Connection;
use diverge_sdk::wire::handshake::{self, Handshook};
use tokio::net::TcpStream;
use tokio::time::{Duration, sleep};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use super::Provider;
use crate::config::outgoing::Outgoing;
use crate::protocol::keepalive;

/// Dial `client` — at its address, as `ws://<address>/` when it carries
/// no scheme — make the handshake as the connector,
/// serve the connection under the peer's identity until it ends, wait
/// five seconds, and dial again: whether the connection ended, the dial
/// failed or the handshake did, and forever. It ends only when the task
/// it runs in is ended. The peer that answers must be the identity
/// `client` names, and no other. The provider
/// announces itself to no peer, and serves the peer's
/// `daemons::announce` as it does for a peer that dialled it, listing
/// it with the address dialled here. The socket has transport keepalive on — see
/// [`keepalive`] — so a peer that vanished without a close is a
/// connection that ends, and a dial again.
pub async fn dial(provider: Arc<Provider>, client: Outgoing) {
    let url = if client.address.contains("://") { client.address.clone() } else { format!("ws://{}/", client.address) };
    loop {
        if let Ok(socket) = keepalive::connect(url.as_str()).await {
            let address = peer_ip(&socket);
            let made = handshake::connect(Connection::Outgoing(socket), &provider.authenticator.dialling(&client.identity), address).await;
            if let Ok(Handshook { connection, peer }) = made {
                provider.held(connection, peer, address, Some(client.address.clone())).await;
            }
        }
        // Five seconds: long enough that a peer that is down is not
        // hammered, short enough that one that is back is served soon.
        sleep(Duration::from_secs(5)).await;
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
