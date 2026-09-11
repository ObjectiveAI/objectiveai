//! One connection, incoming or outgoing.

use std::fmt;
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use bytes::Bytes;
use futures_util::{Sink, Stream};

/// A connection, however it was made.
///
/// # The variant is about the socket, not about the protocol
///
/// [`Incoming`](Self::Incoming) is a connection that came to this
/// process, which its own server upgraded. [`Outgoing`](Self::Outgoing)
/// is one this process went out and made. That is a fact about TCP and
/// says nothing about which half of the protocol is spoken over it.
///
/// Both halves need both. A provider usually waits to be connected to
/// and sometimes goes out to a caller it cannot otherwise reach; a
/// caller usually goes out and can perfectly well be connected to.
/// Neither arrangement changes a single frame — the same scopes, the
/// same channels, the same eleven requests going the same direction.
///
/// The one place it shows is [`auth`](crate::frame::auth): the end
/// whose socket is [`Outgoing`](Self::Outgoing) sends the credential,
/// because whichever side dialled authenticates. That is a fact about
/// the connection rather than about either half.
///
/// # Why one type
///
/// Because the two libraries agree on nothing else. axum hands out a
/// socket with inherent `recv` and `send`; tungstenite hands out a
/// [`Stream`] and a [`Sink`] with neither. Both call a binary payload [`Bytes`], and that is
/// about where it stops.
///
/// So the difference is absorbed here, once. It is a [`Stream`] of
/// payloads and a [`Sink`] of them, and everything above reads and
/// writes without knowing how the socket arrived — including by
/// [`split`](futures_util::StreamExt::split)ting the two apart, which
/// is how more than one scope shares a connection.
///
/// # What it does not do
///
/// Neither dial nor accept. Both variants take a socket somebody else
/// finished making: a provider upgrades a request its own server
/// received, a caller connects with its own tokio-tungstenite. The one
/// thing this crate dials is the proxy inside a container, through
/// [`ContainerClient`](crate::server::container_client::ContainerClient)
/// behind the `server` feature — and that socket never becomes one of
/// these, because the proxy's wire needs a clean close and an abrupt
/// end told apart, which this type folds into one ending.
///
/// Which keeps every question about the transport where it belongs:
/// what the URL is, what the TLS story is, what authenticates the
/// upgrade, and what else that server or process does.
pub enum Connection {
    /// A connection that came in, which this process's own server
    /// upgraded.
    Incoming(axum::extract::ws::WebSocket),
    /// A connection this process went out and made.
    Outgoing(
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
impl Stream for Connection {
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
                Connection::Incoming(socket) => {
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
                            return Poll::Ready(Some(Err(Error::Incoming(
                                error,
                            ))));
                        }
                    }
                }
                Connection::Outgoing(stream) => {
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
                            return Poll::Ready(Some(Err(Error::Outgoing(
                                error,
                            ))));
                        }
                    }
                }
            }
        }
    }
}

/// One binary payload at a time, out.
///
/// Binary always. Nothing in this protocol is text, and a payload that
/// looks like text — a JSON request, an MCP body — is still bytes on
/// the wire.
///
/// # This is what makes splitting possible
///
/// [`StreamExt::split`](futures_util::StreamExt::split) requires
/// `Sink`, and splitting is what the protocol needs: a laboratory run
/// streams a filetree for as long as its container lives, while every
/// other scope on the same connection carries on. One task holding the
/// read half and another holding the write half is the only way that
/// works, and a socket that were only a [`Stream`] could not be taken
/// apart that way.
///
/// Both inner sockets are already `Sink`s. This one would have been
/// narrower than what it wraps.
impl Sink<Bytes> for Connection {
    type Error = Error;

    fn poll_ready(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), Error>> {
        match self.get_mut() {
            Connection::Incoming(socket) => {
                Pin::new(socket).poll_ready(cx).map_err(Error::Incoming)
            }
            Connection::Outgoing(stream) => {
                Pin::new(stream).poll_ready(cx).map_err(Error::Outgoing)
            }
        }
    }

    fn start_send(
        self: Pin<&mut Self>,
        payload: Bytes,
    ) -> Result<(), Error> {
        match self.get_mut() {
            Connection::Incoming(socket) => Pin::new(socket)
                .start_send(axum::extract::ws::Message::Binary(payload))
                .map_err(Error::Incoming),
            Connection::Outgoing(stream) => Pin::new(stream)
                .start_send(tokio_tungstenite::tungstenite::Message::Binary(
                    payload,
                ))
                .map_err(Error::Outgoing),
        }
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), Error>> {
        match self.get_mut() {
            Connection::Incoming(socket) => {
                Pin::new(socket).poll_flush(cx).map_err(Error::Incoming)
            }
            Connection::Outgoing(stream) => {
                Pin::new(stream).poll_flush(cx).map_err(Error::Outgoing)
            }
        }
    }

    /// Close the socket, which sends a close frame.
    ///
    /// The graceful half of a close. Dropping a
    /// [`Connection`] instead is the ungraceful one, and both are
    /// allowed — a peer reads the same `None` either way, because a
    /// close and a stream that stops mean the same thing here.
    fn poll_close(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), Error>> {
        match self.get_mut() {
            Connection::Incoming(socket) => {
                Pin::new(socket).poll_close(cx).map_err(Error::Incoming)
            }
            Connection::Outgoing(stream) => {
                Pin::new(stream).poll_close(cx).map_err(Error::Outgoing)
            }
        }
    }
}

impl fmt::Debug for Connection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Connection::Incoming(_) => f.write_str("Connection::Incoming"),
            Connection::Outgoing(_) => f.write_str("Connection::Outgoing"),
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
    /// An incoming socket failed.
    Incoming(axum::Error),
    /// An outgoing socket failed.
    Outgoing(tokio_tungstenite::tungstenite::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Incoming(error) => write!(f, "websocket failed: {error}"),
            Error::Outgoing(error) => write!(f, "websocket failed: {error}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Incoming(error) => Some(error),
            Error::Outgoing(error) => Some(error),
        }
    }
}
