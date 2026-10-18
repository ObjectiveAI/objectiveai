//! The daemon that accepts connections from other daemons through a
//! provider, one connection at a time.

use std::future::Future;

use futures_util::Stream;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::shared::containers::daemon;
use crate::shared::daemons::Connection;

/// What takes a daemon connection another daemon opened through the
/// provider: the acceptor's session for that connection, which it
/// holds for the connection's life.
///
/// Every time a connector names this daemon on a `daemons::connect`,
/// the provider announces the connection on the accept scope with a
/// [`Connection`] — who, from where, and how they say they are let in
/// — and this is asked for it. Taking it means handing back the stream
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
/// # The credential is the connection's
///
/// The [`Mode`](crate::shared::daemons::Mode) the connection carries
/// is what a socket would have presented as its first frame, judged
/// by whoever implements this as any credential is judged, against
/// the address the provider saw. Nothing is retried.
pub trait Acceptor: Send + Sync {
    /// The server frames the daemon sends on one connection, in order,
    /// then the end.
    type Frames: Stream<Item = daemon::server::Owned> + Send + 'static;

    /// Take the connection, or decline it.
    ///
    /// [`None`] declines: the provider finishes its half with nothing,
    /// and the connector is told it was denied.
    fn accept(
        &self,
        connection: Connection,
        from_connector: UnboundedReceiver<daemon::client::Owned>,
    ) -> impl Future<Output = Option<Self::Frames>> + Send;
}
