//! One socket, whichever end dialled it.

use std::fmt;

use bytes::Bytes;

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
/// [`Stream`](futures_util::Stream) and a [`Sink`](futures_util::Sink)
/// with neither. Both call a binary payload [`Bytes`], and that is
/// about where it stops.
///
/// So the difference is absorbed here, once, and everything above
/// reads and writes payloads without knowing how the socket arrived.
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

impl WebSocket {
    /// The next binary payload, or `None` when the connection ends.
    ///
    /// # What it hides
    ///
    /// Pings and pongs, which are the library's business — axum
    /// answers them itself, and tungstenite does the same. A protocol
    /// with no heartbeat of its own has nothing to say about them.
    ///
    /// A close, and a stream that simply stops, both arrive as `None`.
    /// They are the same thing to everything above: the connection is
    /// over, so every scope on it is over, and there is nobody left to
    /// tell.
    ///
    /// # What it does not hide
    ///
    /// A text message, which becomes [`Error::NotBinary`]. Every
    /// message in this protocol is binary, so one that is not is a
    /// peer that disagrees about the protocol — and swallowing it
    /// would leave this end waiting on a connection that is not going
    /// to work.
    pub async fn recv(&mut self) -> Option<Result<Bytes, Error>> {
        loop {
            match self {
                WebSocket::Accepted(socket) => match socket.recv().await? {
                    Ok(axum::extract::ws::Message::Binary(bytes)) => {
                        return Some(Ok(bytes));
                    }
                    Ok(axum::extract::ws::Message::Text(_)) => {
                        return Some(Err(Error::NotBinary));
                    }
                    Ok(axum::extract::ws::Message::Close(_)) => return None,
                    // A ping or a pong. The library has already dealt
                    // with it.
                    Ok(_) => continue,
                    Err(error) => return Some(Err(Error::Accepted(error))),
                },
                WebSocket::Dialled(stream) => {
                    use futures_util::StreamExt as _;
                    use tokio_tungstenite::tungstenite::Message;
                    match stream.next().await? {
                        Ok(Message::Binary(bytes)) => return Some(Ok(bytes)),
                        Ok(Message::Text(_)) => {
                            return Some(Err(Error::NotBinary));
                        }
                        Ok(Message::Close(_)) => return None,
                        // A ping, a pong, or a raw frame.
                        Ok(_) => continue,
                        Err(error) => {
                            return Some(Err(Error::Dialled(error)));
                        }
                    }
                }
            }
        }
    }

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

/// A socket that would not carry a payload.
#[derive(Debug)]
pub enum Error {
    /// An accepted socket failed.
    Accepted(axum::Error),
    /// A dialled socket failed.
    Dialled(tokio_tungstenite::tungstenite::Error),
    /// A text message arrived, and this protocol has none.
    ///
    /// Not a transport failure — the socket is fine and the peer is
    /// not. Kept separate from the two above because the response
    /// differs: a broken socket is over, and a peer talking nonsense
    /// may simply be a peer this end should stop talking to.
    NotBinary,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Accepted(error) => write!(f, "websocket failed: {error}"),
            Error::Dialled(error) => write!(f, "websocket failed: {error}"),
            Error::NotBinary => {
                f.write_str("a text message, where every message is binary")
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Accepted(error) => Some(error),
            Error::Dialled(error) => Some(error),
            Error::NotBinary => None,
        }
    }
}
