//! The database that lives with the caller, dialed for a container.

use std::future::Future;

use bytes::Bytes;
use futures_util::Stream;
use tokio::sync::mpsc::UnboundedReceiver;

/// How a caller carries a container's database connection to a
/// database it holds.
///
/// Something inside the container dialed Postgres; the provider
/// carried the connection out as a pair of channels correlated by a
/// connection id, and the executor hands both halves here: what the
/// container writes arrives on `from_container`, verbatim, one piece
/// per frame, and ends when the container's socket does; what the
/// database says goes back as the returned stream, likewise verbatim,
/// and its end is the database closing the connection. Nothing is
/// parsed — TLS negotiation and every protocol extension cross
/// untouched, and a message larger than one frame spans several.
///
/// `None` declines the dial: the executor finishes the provider's
/// channel with nothing, and the container's socket is shut. pgwire
/// is client-first, so the first bytes on `from_container` are the
/// container's startup message, already waiting.
pub trait PostgresDialer: Send + Sync {
    /// What the database says, verbatim; its end is the connection
    /// closed.
    type Connection: Stream<Item = Bytes> + Send + 'static;

    /// Dial for one connection, or decline.
    fn dial(
        &self,
        connection_id: u32,
        from_container: UnboundedReceiver<Bytes>,
    ) -> impl Future<Output = Option<Self::Connection>> + Send;
}
