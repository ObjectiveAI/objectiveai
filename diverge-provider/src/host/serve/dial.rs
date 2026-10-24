//! One peer the provider dials, for the provider's life.

use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;

use diverge_sdk::wire::connection::Connection;
use diverge_sdk::wire::handshake::{self, Handshook};
use tokio::net::TcpStream;
use tokio::time::{Duration, sleep};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

use super::Provider;
use crate::config::clients::Client;

/// Dial `client` — at its address, as `ws://<address>/` when it carries
/// no scheme — make the handshake as the connector from its issuer,
/// serve the connection under the peer's identity until it ends, wait
/// five seconds, and dial again: whether the connection ended, the dial
/// failed or the handshake did, and forever. It ends only when the task
/// it runs in is ended. The peer is judged as a peer that dialled the
/// provider is: its identity must be one `accepted` lists.
pub async fn dial(provider: Arc<Provider>, client: Client) {
    let url = if client.address.contains("://") { client.address.clone() } else { format!("ws://{}/", client.address) };
    loop {
        if let Ok((socket, _)) = connect_async(url.as_str()).await {
            let address = peer_ip(&socket);
            let made = handshake::connect(Connection::Outgoing(socket), &client.issuer, provider.authenticator.as_ref(), address).await;
            if let Ok(Handshook { connection, peer }) = made {
                provider.held(connection, peer, address).await;
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
