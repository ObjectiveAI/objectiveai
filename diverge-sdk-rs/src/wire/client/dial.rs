//! A daemon, dialled.

use std::fmt;
use std::future::Future;
use std::net::{IpAddr, Ipv4Addr};

use futures_util::StreamExt as _;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

use super::handle::Handle;
use super::router::Router;
use crate::wire::connection::Connection;
use crate::wire::handshake::{Authenticator, Handshook, Peer, connect};

/// The daemon at `address` dialled, the [`handshake`](crate::wire::handshake)
/// done as its connector — this end's token from `issuer` presented,
/// the daemon's own from the same issuer verified, both through
/// `authenticator` — and the connection split: the handle every request
/// is put on, the router's run — the read loop, not yet running — and
/// the daemon, verified. The caller spawns the run on its own runtime,
/// and its completion is the connection's end, however it ended.
///
/// The address is dialled as given when it carries a scheme —
/// `wss://daemon.example/` — and as `ws://<address>/` otherwise. A
/// socket that could not be made is [`DialError::Dial`]; a handshake
/// that did not complete, on either end's word, is
/// [`DialError::Handshake`].
pub async fn dial(
    address: &str,
    issuer: &str,
    authenticator: &dyn Authenticator,
) -> Result<(Handle, impl Future<Output = ()> + Send + 'static, Peer), DialError> {
    let (socket, _) = connect_async(url(address)).await.map_err(|error| DialError::Dial(error.to_string()))?;
    let at = peer_ip(&socket);
    let Handshook { connection, peer } =
        connect(Connection::Outgoing(socket), issuer, authenticator, at).await.map_err(|error| DialError::Handshake(error.to_string()))?;
    let (sink, stream) = connection.split();
    let (registration_sender, registration_receiver) = mpsc::unbounded_channel();
    let (closed_sender, closed_receiver) = mpsc::unbounded_channel();
    let router = Router::new(stream, registration_receiver, closed_sender);
    let run = async move {
        let _ = router.run().await;
    };
    Ok((Handle::new(sink, registration_sender, closed_receiver), run, peer))
}

/// The URL for `address`: the address itself when it carries a
/// scheme, and otherwise `ws://` in front of it and `/` behind.
pub fn url(address: &str) -> String {
    if address.contains("://") { address.to_string() } else { format!("ws://{address}/") }
}

/// The peer's address, off the TCP stream under the WebSocket, for the
/// authenticator to judge the daemon's token by. The unspecified
/// address for a stream that will not say.
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
    /// The socket was made and the handshake did not complete: this
    /// end had no token, the daemon closed, or its answer was not
    /// accepted.
    Handshake(String),
}

impl fmt::Display for DialError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DialError::Dial(why) => write!(f, "the daemon could not be dialled: {why}"),
            DialError::Handshake(why) => write!(f, "the handshake did not complete: {why}"),
        }
    }
}

impl std::error::Error for DialError {}
