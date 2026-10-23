//! A daemon, dialled as an account.

use std::fmt;
use std::future::Future;
use std::net::{IpAddr, Ipv4Addr};

use futures_util::StreamExt as _;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

use super::authorization::{Auth, Authorization};
use super::authorize::authorize;
use super::authorized::Authorized;
use super::handle::Handle;
use super::refusing::Refusing;
use super::router::Router;
use crate::wire::connection::Connection;

/// The daemon at `address` dialled, `key` presented as the one
/// credential frame, and the connection split: the handle every
/// request is put on, and the router's run — the read loop, not yet
/// running. The caller spawns it on its own runtime, and its
/// completion is the connection's end, however the connection ended.
///
/// The address is dialled as given when it carries a scheme —
/// `wss://daemon.example/` — and as `ws://<address>/` otherwise. A
/// socket that could not be made is [`DialError::Dial`]; a credential
/// that could not be presented is [`DialError::Authorize`]. The daemon
/// answers the credential with nothing, admitting or closing, so a key
/// it refuses is seen as the first request's scope closing.
pub async fn dial(address: &str, key: &str) -> Result<(Handle, impl Future<Output = ()> + Send + 'static), DialError> {
    let (socket, _) = connect_async(url(address)).await.map_err(|error| DialError::Dial(error.to_string()))?;
    let peer = peer_ip(&socket);
    let presented = Authorization::<Refusing>::Outgoing { auth: Auth::Unbrokered(key.to_string()), provider_identity: address.to_string() };
    let Authorized { connection, .. } =
        authorize(Connection::Outgoing(socket), presented, peer).await.map_err(|error| DialError::Authorize(format!("{error:?}")))?;
    let (sink, stream) = connection.split();
    let (registration_sender, registration_receiver) = mpsc::unbounded_channel();
    let (closed_sender, closed_receiver) = mpsc::unbounded_channel();
    let router = Router::new(stream, registration_receiver, closed_sender);
    let run = async move {
        let _ = router.run().await;
    };
    Ok((Handle::new(sink, registration_sender, closed_receiver), run))
}

/// The URL for `address`: the address itself when it carries a
/// scheme, and otherwise `ws://` in front of it and `/` behind.
pub fn url(address: &str) -> String {
    if address.contains("://") { address.to_string() } else { format!("ws://{address}/") }
}

/// The peer's address, off the TCP stream under the WebSocket; the
/// handshake takes it, and on a connection this end dialled nothing
/// reads it. The unspecified address for a stream that will not say.
fn peer_ip(socket: &WebSocketStream<MaybeTlsStream<TcpStream>>) -> IpAddr {
    match socket.get_ref() {
        MaybeTlsStream::Plain(stream) => stream.peer_addr().map(|address| address.ip()).unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED)),
        _ => IpAddr::V4(Ipv4Addr::UNSPECIFIED),
    }
}

/// Why [`dial`] made no connection: no socket, or a handshake that did
/// not complete.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialError {
    /// The socket could not be made: the address did not resolve,
    /// nothing listens there, or the WebSocket upgrade was refused.
    Dial(String),
    /// The socket was made and the credential could not be presented
    /// on it.
    Authorize(String),
}

impl fmt::Display for DialError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DialError::Dial(why) => write!(f, "the daemon could not be dialled: {why}"),
            DialError::Authorize(why) => write!(f, "the credential could not be presented: {why}"),
        }
    }
}

impl std::error::Error for DialError {}
