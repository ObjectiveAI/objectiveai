//! One socket, whichever end dialled it.

use std::fmt;
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use bytes::Bytes;
use futures_util::Stream;

/// A connection, however it was made.
///
/// # The variant is about the socket, not about the protocol
///
/// [`Accepted`](Self::Accepted) is one this process's own server
/// upgraded. [`Dialled`](Self::Dialled) is one this process opened.
/// That is a fact about TCP and says nothing about which half of the
/// protocol is spoken over it.
///
/// Both halves need both. A provider usually waits to be dialled and
/// sometimes dials a caller that cannot be reached otherwise; a caller
/// usually dials and can perfectly well be dialled into. Neither
/// arrangement changes a single frame — the same scopes, the same
/// channels, the same ten requests going the same direction.
///
/// The one place it shows is [`auth`](crate::frame::auth): whichever
/// side DIALLED sends the credential, and that is a fact about the
/// connection rather than about either half.
///
/// # Why one type
///
/// Because the two libraries agree on nothing else. axum hands out a
/// socket with inherent `recv` and `send`; tungstenite hands out a
/// [`Stream`] and a [`Sink`](futures_util::Sink) with neither. Both call a binary payload [`Bytes`], and that is
/// about where it stops.
///
/// So the difference is absorbed here, once. It is a
/// [`Stream`] of payloads and has a [`send`](Self::send), and
/// everything above reads and writes without knowing how the socket
/// arrived.
///
/// # What it does not do
///
/// Neither dial nor accept. Both variants take a socket somebody else
/// finished making: a provider upgrades a request its own server
/// received, a dialler connects with its own tokio-tungstenite. This
/// crate carries `tokio-tungstenite` with `stream` alone — enough to
/// name the type and not to connect with — and `axum` without `http1`,
/// for the same reason.
///
/// Which keeps every question about the transport where it belongs:
/// what the URL is, what the TLS story is, what authenticates the
/// upgrade, and what else that server or process does.
pub enum WebSocket {
    /// A socket this process's own server upgraded.
    Accepted(axum::extract::ws::WebSocket),
    /// A socket this process dialled.
    Dialled(
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    ),
}

/// The binary payloads, one at a time, until the connection ends.
///
/// # Everything that is not a payload is skipped
///
/// Pings and pongs, which are the library's business — axum answers
/// them itself, and tungstenite does the same. A protocol with no
/// heartbeat of its own has nothing to say about them.
///
/// And text, which this protocol has none of. Skipped rather than
/// reported: a peer sending one has not necessarily stopped sending
/// the ones that matter, and a reader that gave up would be throwing
/// away the frames it came for. Something else on that socket may
/// simply not be this protocol.
///
/// A close, and a stream that simply stops, both end this one. They
/// are the same thing to everything above: the connection is over, so
/// every scope on it is over, and there is nobody left to tell.
///
/// The item is a payload rather than a frame. Decoding is
/// [`ClientFrame`](crate::frame::client::ClientFrame)'s and
/// [`ServerFrame`](crate::frame::server::ServerFrame)'s, and which of
/// the two a reader wants is not something a socket knows.
impl Stream for WebSocket {
    type Item = Result<Bytes, Error>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        // Both sockets are `Unpin` — a `TcpStream` is, and so is
        // hyper's upgraded one — so this never has to project.
        let this = self.get_mut();
        loop {
            match this {
                WebSocket::Accepted(socket) => {
                    use axum::extract::ws::Message;
                    match ready!(Pin::new(&mut *socket).poll_next(cx)) {
                        Some(Ok(Message::Binary(bytes))) => {
                            return Poll::Ready(Some(Ok(bytes)));
                        }
                        Some(Ok(Message::Close(_))) | None => {
                            return Poll::Ready(None);
                        }
                        // A ping, a pong, or text. None of them are
                        // this protocol's.
                        Some(Ok(_)) => continue,
                        Some(Err(error)) => {
                            return Poll::Ready(Some(Err(Error::Accepted(
                                error,
                            ))));
                        }
                    }
                }
                WebSocket::Dialled(stream) => {
                    use tokio_tungstenite::tungstenite::Message;
                    match ready!(Pin::new(&mut *stream).poll_next(cx)) {
                        Some(Ok(Message::Binary(bytes))) => {
                            return Poll::Ready(Some(Ok(bytes)));
                        }
                        Some(Ok(Message::Close(_))) | None => {
                            return Poll::Ready(None);
                        }
                        // A ping, a pong, text, or a raw frame. None
                        // of them are this protocol's.
                        Some(Ok(_)) => continue,
                        Some(Err(error)) => {
                            return Poll::Ready(Some(Err(Error::Dialled(
                                error,
                            ))));
                        }
                    }
                }
            }
        }
    }
}

impl WebSocket {
    /// Send one binary payload.
    ///
    /// Binary always. Nothing in this protocol is text, and a payload
    /// that looks like text — a JSON request, an MCP body — is still
    /// bytes on the wire.
    pub async fn send(&mut self, payload: Bytes) -> Result<(), Error> {
        match self {
            WebSocket::Accepted(socket) => socket
                .send(axum::extract::ws::Message::Binary(payload))
                .await
                .map_err(Error::Accepted),
            WebSocket::Dialled(stream) => {
                use futures_util::SinkExt as _;
                stream
                    .send(tokio_tungstenite::tungstenite::Message::Binary(
                        payload,
                    ))
                    .await
                    .map_err(Error::Dialled)
            }
        }
    }
}

impl fmt::Debug for WebSocket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WebSocket::Accepted(_) => f.write_str("WebSocket::Accepted"),
            WebSocket::Dialled(_) => f.write_str("WebSocket::Dialled"),
        }
    }
}

/// A socket that failed.
///
/// Only the transport. Nothing a peer can put in a message reaches
/// here — a payload this end cannot read is a payload, and what it
/// means is decided further up.
#[derive(Debug)]
pub enum Error {
    /// An accepted socket failed.
    Accepted(axum::Error),
    /// A dialled socket failed.
    Dialled(tokio_tungstenite::tungstenite::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Accepted(error) => write!(f, "websocket failed: {error}"),
            Error::Dialled(error) => write!(f, "websocket failed: {error}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Accepted(error) => Some(error),
            Error::Dialled(error) => Some(error),
        }
    }
}
