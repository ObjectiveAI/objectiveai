//! The daemon, dialled as an account.

use std::net::{IpAddr, Ipv4Addr};

use diverge_sdk::wire::client::authorization::{Auth, Authorization};
use diverge_sdk::wire::client::authorize::authorize;
use diverge_sdk::wire::client::authorized::Authorized;
use diverge_sdk::wire::client::handle::Handle;
use diverge_sdk::wire::client::router::Router;
use diverge_sdk::wire::connection::Connection;
use futures_util::StreamExt as _;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

use super::{Error, Refusing};

/// The daemon at `address` dialled, the account's `key` presented as
/// the one credential frame, and the connection split: the router
/// reading on a task of its own, the handle every request is put on.
/// The address is dialled as given when it carries a scheme —
/// `wss://daemon.example/` — and as `ws://<address>/` otherwise. A
/// socket that could not be made is [`Error::Dial`]; the daemon
/// answers the credential with nothing, admitting or closing, so a
/// key it refuses is seen as the first request's scope closing.
pub async fn dial(address: &str, key: &str) -> Result<Handle, Error> {
    let (socket, _) = connect_async(url(address))
        .await
        .map_err(|error| Error::Dial(error.to_string()))?;
    let peer = peer_ip(&socket);
    let presented = Authorization::<Refusing>::Outgoing {
        auth: Auth::Unbrokered(key.to_string()),
        provider_identity: address.to_string(),
    };
    let Authorized { connection, .. } = authorize(Connection::Outgoing(socket), presented, peer)
        .await
        .map_err(|error| Error::Authorize(format!("{error:?}")))?;
    let (sink, stream) = connection.split();
    let (registration_sender, registration_receiver) = mpsc::unbounded_channel();
    let (closed_sender, closed_receiver) = mpsc::unbounded_channel();
    let router = Router::new(stream, registration_receiver, closed_sender);
    tokio::spawn(async move {
        let _ = router.run().await;
    });
    Ok(Handle::new(sink, registration_sender, closed_receiver))
}

/// The URL for `address`: the address itself when it carries a
/// scheme, and otherwise `ws://` in front of it and `/` behind.
fn url(address: &str) -> String {
    if address.contains("://") {
        address.to_string()
    } else {
        format!("ws://{address}/")
    }
}

/// The peer's address, off the TCP stream under the WebSocket; the
/// handshake takes it, and on a connection this end dialled nothing
/// reads it. The unspecified address for a stream that will not say.
fn peer_ip(socket: &WebSocketStream<MaybeTlsStream<TcpStream>>) -> IpAddr {
    match socket.get_ref() {
        MaybeTlsStream::Plain(stream) => stream
            .peer_addr()
            .map(|address| address.ip())
            .unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED)),
        _ => IpAddr::V4(Ipv4Addr::UNSPECIFIED),
    }
}
