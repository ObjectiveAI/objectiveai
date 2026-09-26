//! Dialling the proxy inside a container: the one place this crate
//! dials.

use std::fmt;

use futures_util::StreamExt as _;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite;

use crate::wire::client::handle::Handle;
use crate::wire::client::router::Router;
use crate::wire::connection::Connection;

/// Open the one WebSocket to a container's proxy and hand back the
/// frame-level client that speaks on it.
///
/// `address` is what a [`Container`](super::container::Container)
/// reports — `ws://host:14979`, the proxy's
/// [`OUTSIDE_PORT`](crate::container_proxy::outside::OUTSIDE_PORT)
/// at wherever the provider reaches it — and is dialled as given, at
/// its root path. The socket becomes an
/// [`Outgoing`](Connection::Outgoing) connection, split in two: the
/// read half runs as a [`Router`] on a task of its own for as long as
/// the socket lives, and the write half is the [`Handle`] returned,
/// which every executor under
/// [`container_proxy_endpoints`](crate::container_proxy::outside)
/// takes to open its scope.
///
/// # No auth frame
///
/// Nothing is presented and nothing is judged: the wire has no
/// handshake, by its own rule. A proxy that sends an auth frame is
/// speaking some other protocol, and the router ends on it.
///
/// # Why this crate dials here and nowhere else
///
/// Everywhere else a socket is somebody else's to make: a provider
/// upgrades a request its own server received, a caller connects with
/// its own client. The proxy is this crate's own wire, and the
/// address is the deployer's answer, so there is nothing for the
/// provider to compose — which is what makes the dial the crate's.
pub async fn dial(address: &str) -> Result<Handle, DialError> {
    let (socket, _) = tokio_tungstenite::connect_async(address)
        .await
        .map_err(DialError::Connect)?;
    let (sink, stream) = Connection::Outgoing(socket).split();
    let (registration_sender, registration_receiver) = mpsc::unbounded_channel();
    let (closed_sender, closed_receiver) = mpsc::unbounded_channel();
    let router = Router::new(stream, registration_receiver, closed_sender);
    tokio::spawn(async move {
        // An auth frame from the proxy ends the router, which ends
        // every scope on the connection; the socket is dropped with
        // it, and there is nobody to report to.
        let _ = router.run().await;
    });
    Ok(Handle::new(sink, registration_sender, closed_receiver))
}

/// A proxy that could not be dialled.
#[derive(Debug)]
pub enum DialError {
    /// The connection was not made: refused, unreachable, or the
    /// upgrade refused.
    Connect(tungstenite::Error),
}

impl fmt::Display for DialError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DialError::Connect(error) => {
                write!(f, "the proxy could not be dialled: {error}")
            }
        }
    }
}

impl std::error::Error for DialError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DialError::Connect(error) => Some(error),
        }
    }
}
