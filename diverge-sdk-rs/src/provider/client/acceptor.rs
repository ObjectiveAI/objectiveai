//! The daemon that accepts connections from other daemons through a
//! provider, one connection at a time.

use std::future::Future;

use futures_util::Stream;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::shared::daemons::{Connection, client, server};

/// What takes a daemon connection another daemon opened through the
/// provider: the acceptor's session for that connection, which it
/// holds for the connection's life.
///
/// Every time a connector names this daemon on a `daemons::connect`,
/// the provider announces the connection on the accept scope with a
/// [`Connection`] — who to the provider, and from where — and this is
/// asked for it. Taking it means handing back the stream
/// of server frames the daemon sends on it; declining means handing
/// back [`None`], which finishes the provider's half with nothing and
/// refuses the connector. See
/// [`daemons`](crate::shared::daemons) for what crosses.
///
/// # Both directions
///
/// `from_connector` carries every client frame the connector sends,
/// in order, and ends when the connector is gone; the stream carries
/// every server frame the daemon sends, in order, and its end hangs
/// the connection up. Neither side is read by anything between: the
/// connector mints the scopes and the channels, as a client of the
/// daemon does, and the numbers mean nothing outside the connection.
///
/// # The handshake is the connection's
///
/// The relayed connection begins as one on a socket does, with the
/// [`handshake`](crate::wire::handshake): the first frame on
/// `from_connector` is the connector's token, and the first frame the
/// stream sends must be this daemon's own, whatever issuer the
/// connector's is from — the connectee's half, judged by whoever implements this against the
/// address the provider saw. A connector not accepted is one the stream
/// ends on, sending nothing. Nothing is retried.
///
/// # The stream is handed back before the first frame
///
/// Nothing reaches `from_connector` until [`accept`](Self::accept)
/// has answered: the provider relays a connection only once it is
/// taken, and it is taken by the half this end opens when the stream
/// is handed back. So the stream is handed back first, and the
/// connector's token read behind it, on a task of the connection's
/// own. An `accept` that waits on `from_connector` waits on the half
/// that waits on it, and never answers. [`None`] is for what the
/// [`Connection`] alone decides.
pub trait Acceptor: Send + Sync {
    /// The server frames the daemon sends on one connection, in order,
    /// then the end.
    type Frames: Stream<Item = server::Owned> + Send + 'static;

    /// Take the connection, or decline it.
    ///
    /// [`None`] declines: the provider finishes its half with nothing,
    /// and the connector is told it was denied.
    fn accept(
        &self,
        connection: Connection,
        from_connector: UnboundedReceiver<client::Owned>,
    ) -> impl Future<Output = Option<Self::Frames>> + Send;
}
