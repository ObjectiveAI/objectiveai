//! One peer the provider dials, for the provider's life.

use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;

use diverge_sdk::wire::connection::Connection;
use diverge_sdk::wire::server::authorization::{Auth, Authorization};
use tokio::net::TcpStream;
use tokio::time::{Duration, sleep};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

use super::Provider;
use crate::config::clients::Unbrokered;

/// Dial `peer` at `ws://<address>/`, present its key as the first
/// frame under its identity, serve the connection until it ends,
/// wait five seconds, and dial again — whether the connection ended
/// or the dial failed, and forever: this ends only when the task it
/// runs in is ended.
pub async fn dial(provider: Arc<Provider>, peer: Unbrokered) {
    let url = format!("ws://{}/", peer.address);
    loop {
        if let Ok((socket, _)) = connect_async(url.as_str()).await {
            let address = peer_ip(&socket);
            let authorization = Authorization::Outgoing {
                auth: Auth::Unbrokered(peer.key.clone()),
                client_identity: peer.identity.clone(),
            };
            provider.connection(Connection::Outgoing(socket), authorization, address).await;
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
