//! The proxy inside a container, from the server's side.

use std::fmt;

use bytes::Bytes;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use crate::encode::{Encode, Writer};

/// A socket to one path of the proxy: tungstenite's own, not a
/// [`Connection`](crate::connection::Connection), because the proxy's
/// wire lives on the difference between a clean close and an abrupt
/// end and that type folds the two into one ending.
pub(crate) type WebSocket = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// The proxy beside a container: where it is, held once, dialed per
/// path.
///
/// Every executor under [`container_proxy`](crate::container_proxy)
/// takes one of these and opens the path it serves — `/requests` to
/// hear the container's asks, an answer path to answer one, the
/// filetree, a read, a write. Nothing is held open between calls:
/// a path is a socket, opened when asked for and closed when done,
/// which is the wire's own shape.
///
/// # The URL is the caller's
///
/// `ws://<host>:14979` — the proxy's
/// [`PORT`](crate::container_proxy::PORT) at wherever the provider
/// can reach the container, which is a fact about the provider's
/// network and not this crate's. It is kept as given and a path is
/// appended to it; a trailing slash is dropped so the paths, which
/// begin with one, do not double it.
///
/// # The one place this crate dials
///
/// Everywhere else a socket is somebody else's to make. This is the
/// exception because the proxy is this crate's own wire, dialed by
/// the same code that defines it, and there is nothing a provider
/// could usefully decide about how.
#[derive(Debug, Clone)]
pub struct ContainerClient {
    /// The proxy's base, without a trailing slash.
    url: String,
}

impl ContainerClient {
    /// Where the proxy is. No I/O happens here.
    pub fn new(url: impl Into<String>) -> Self {
        let url = url.into();
        let trimmed = url.trim_end_matches('/').len();
        let mut url = url;
        url.truncate(trimmed);
        ContainerClient { url }
    }

    /// Open one path.
    ///
    /// A non-`101` answer is [`Refused`](OpenError::Refused) with the
    /// status: the proxy's `404` for a channel it does not know and
    /// `409` for one already being answered, or `/requests` already
    /// taken.
    pub(crate) async fn open(&self, path: &str) -> Result<WebSocket, OpenError> {
        let url = format!("{}{}", self.url, path);
        match tokio_tungstenite::connect_async(url).await {
            Ok((socket, _)) => Ok(socket),
            Err(tungstenite::Error::Http(response)) => {
                Err(OpenError::Refused(response.status().as_u16()))
            }
            Err(error) => Err(OpenError::Connect(error)),
        }
    }
}

/// One frame, as the bytes a message carries.
pub(crate) fn encoded<T: Encode>(frame: &T) -> Result<Bytes, T::Error> {
    let mut out = Vec::new();
    frame.encode(&mut Writer::new(&mut out))?;
    Ok(out.into())
}

/// Close cleanly, and wait for the far side to agree.
///
/// The Close frame goes out, and the socket is read until the proxy's
/// own Close comes back or the connection ends — so the clean close
/// has reached the proxy before this returns, rather than being a
/// frame in a buffer the drop may or may not deliver.
pub(crate) async fn finish(mut socket: WebSocket) -> Result<(), tungstenite::Error> {
    socket.close(None).await?;
    drain(&mut socket).await
}

/// Read until the connection ends, after a Close went out.
pub(crate) async fn drain<S>(socket: &mut S) -> Result<(), tungstenite::Error>
where
    S: futures_util::Stream<Item = Result<tungstenite::Message, tungstenite::Error>>
        + Unpin,
{
    use futures_util::StreamExt as _;
    loop {
        match socket.next().await {
            Some(Ok(_)) => {}
            Some(Err(
                tungstenite::Error::ConnectionClosed
                | tungstenite::Error::AlreadyClosed,
            ))
            | None => return Ok(()),
            Some(Err(error)) => return Err(error),
        }
    }
}

/// A path that could not be opened.
#[derive(Debug)]
pub enum OpenError {
    /// The proxy answered the upgrade with a status, carried here: a
    /// `404` is a channel it does not know, a `409` one already being
    /// answered — or `/requests` already held by another server.
    Refused(u16),
    /// The socket could not be made or upgraded.
    Connect(tungstenite::Error),
}

impl fmt::Display for OpenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OpenError::Refused(status) => {
                write!(f, "the proxy refused the path with status {status}")
            }
            OpenError::Connect(error) => {
                write!(f, "the proxy could not be dialed: {error}")
            }
        }
    }
}

impl std::error::Error for OpenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            OpenError::Connect(error) => Some(error),
            OpenError::Refused(_) => None,
        }
    }
}
