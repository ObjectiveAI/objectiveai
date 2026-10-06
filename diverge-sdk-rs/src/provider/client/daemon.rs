//! The daemon a container's program connects to, one connection at a
//! time.

use std::future::Future;

use futures_util::Stream;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::shared::containers::daemon;

/// What takes a daemon connection a container's program opened: the
/// daemon's session for that connection, which the caller holds for
/// the connection's life.
///
/// Every time a program in the container dials its proxy's `/daemon`,
/// the provider announces the connection under an id and this is asked
/// for it. Taking it means handing back the stream of server frames the
/// daemon sends on it; declining means handing back [`None`], which
/// finishes the provider's half with nothing and closes the program's
/// socket. See [`daemon`](crate::shared::containers::daemon) for the
/// pair of channels a connection is.
///
/// # Both directions
///
/// `from_program` carries every client frame the program sends, in
/// order, and ends when the program's socket ends; the stream carries
/// every server frame the daemon sends, in order, and its end closes
/// the program's socket. Neither side is read by anything between: the
/// program mints the scopes and the channels, as a client of the daemon
/// does, and the numbers mean nothing outside the connection.
///
/// # The account is the container's
///
/// Nothing here carries a credential, and the connection presents
/// none: the session is served for the container's
/// [`account`](crate::daemon::create::Inner::account), which the caller
/// knows from the run. Two connections of one container are two
/// sessions, with scopes of their own.
pub trait Daemon: Send + Sync {
    /// The server frames the daemon sends on one connection, in order,
    /// then the end.
    type Frames: Stream<Item = daemon::server::Owned> + Send + 'static;

    /// Take the connection, or decline it.
    ///
    /// [`None`] declines: the provider finishes its half with nothing,
    /// and the program's socket is closed. Nothing is retried.
    fn connect(
        &self,
        connection_id: u32,
        from_program: UnboundedReceiver<daemon::client::Owned>,
    ) -> impl Future<Output = Option<Self::Frames>> + Send;
}
