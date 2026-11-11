//! Transport keepalive on every TCP socket the provider holds: the
//! ones its port accepts and the ones it dials.
//!
//! A peer that vanishes without closing — a machine powered off, a
//! cable pulled, a NAT that forgot the mapping — leaves a socket that
//! is open on this end and never hears another byte, and everything a
//! connection holds would be held with it for as long as the provider
//! runs. TCP keepalive is the transport's own answer: after `IDLE` of
//! silence the kernel probes the peer every `INTERVAL`, and a peer
//! that answers none of the probes is a socket the kernel resets,
//! which the read side sees as the connection ending. It is not a
//! deadline on anything the protocol does — a connection with nothing
//! to say for a week stays up for a week, so long as the peer is
//! there.

use std::time::Duration;

use socket2::{SockRef, TcpKeepalive};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite;
use tokio_tungstenite::tungstenite::client::IntoClientRequest as _;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

/// How long a socket is silent before the first probe.
pub const IDLE: Duration = Duration::from_secs(60);

/// How long between one probe and the next, once probing.
pub const INTERVAL: Duration = Duration::from_secs(15);

/// Turn keepalive on for `stream`, at [`IDLE`] and [`INTERVAL`]. A
/// socket that refuses the option is served without it: the option is
/// what notices a vanished peer sooner, and its absence is no reason
/// to refuse a connection.
pub fn keepalive(stream: &TcpStream) {
    let _ = SockRef::from(stream).set_tcp_keepalive(&TcpKeepalive::new().with_time(IDLE).with_interval(INTERVAL));
}

/// Dial `url` as [`connect_async`](tokio_tungstenite::connect_async)
/// does — the TCP connection to the URL's host and port, `80` when it
/// names none, and the WebSocket upgrade on it — with keepalive turned
/// on between the two. Only `ws://`: the crate speaks no TLS, and a
/// `wss://` URL is refused as `connect_async` refuses it here.
pub async fn connect(url: &str) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>, tungstenite::Error> {
    let request = url.into_client_request()?;
    let port = match request.uri().scheme_str() {
        Some("ws") => request.uri().port_u16().unwrap_or(80),
        Some("wss") => return Err(tungstenite::Error::Url(tungstenite::error::UrlError::TlsFeatureNotEnabled)),
        _ => return Err(tungstenite::Error::Url(tungstenite::error::UrlError::UnsupportedUrlScheme)),
    };
    let host = request.uri().host().ok_or(tungstenite::Error::Url(tungstenite::error::UrlError::NoHostName))?;
    let socket = TcpStream::connect(format!("{host}:{port}")).await.map_err(tungstenite::Error::Io)?;
    keepalive(&socket);
    let (socket, _) = tokio_tungstenite::client_async(request, MaybeTlsStream::Plain(socket)).await?;
    Ok(socket)
}
