//! A watch that is running, and the way to end it.

use crate::client::handle::{Handle, SendError};
use crate::encode::{Encode, Writer};
use crate::endpoints::volumes::watch::client::channel_request;

/// A running watch, and the one thing a caller can say about it.
///
/// Half of what `execute` gives back. The other half
/// is an [`ExecuteStream`](super::ExecuteStream) of the changes, and the
/// split is the same one
/// [`containers::tools::connect`](crate::endpoints::containers::tools::connect)
/// makes: the stream is what the provider says, and this is how a
/// caller says anything back.
///
/// There is one thing to say, so there is one method.
///
/// # Dropping it does nothing
///
/// Which it used to. This type carried a [`Drop`] that sent the
/// disconnect, and that was replaced by [`disconnect`](Self::disconnect)
/// being a method a caller calls.
///
/// A destructor could not await, could not report a failure, and could
/// not be skipped when a caller wanted the watch to outlive the value —
/// so ending a watch was something that happened to a caller rather than
/// something it did. Now a watch ends when somebody says so.
///
/// What that costs is that forgetting is possible again. A caller that
/// drops this without disconnecting leaves the provider walking a tree
/// nobody is listening about, until the connection goes.
#[derive(Debug, Clone)]
pub struct ExecuteHandle {
    /// What the disconnect goes out over.
    ///
    /// A [`Handle`] rather than a pre-encoded frame. That mattered more
    /// when a destructor sent it — a frame carries a scope number it
    /// cannot re-check — and it still matters:
    /// [`send_channel_request`](Handle::send_channel_request) takes back
    /// what the router has closed and then looks the scope up, so a
    /// watch that has already ended sends nothing rather than
    /// disconnecting whoever holds that number now.
    handle: Handle,
    /// The scope this watch is running in, as this end numbered it.
    scope: u32,
}

impl ExecuteHandle {
    /// Take the pieces, from the `execute` that has
    /// them.
    ///
    /// Not public. A watch exists because a request went out, so the
    /// only thing that can honestly make one of these is the thing that
    /// sent it.
    pub(super) fn new(handle: Handle, scope: u32) -> Self {
        ExecuteHandle { handle, scope }
    }

    /// Stop watching.
    ///
    /// Returns when the frame has been written, and that is all it
    /// waits for. What ANSWERS a disconnect is the scope's own finish,
    /// which arrives on the
    /// [`ExecuteStream`](super::ExecuteStream) as [`None`] — so a caller
    /// that wants to see the watch actually end reads the stream until
    /// it does, and one that only wants to stop asking is finished
    /// here.
    ///
    /// # It is the ordinary way to be done
    ///
    /// There is no frame for cancelling a SCOPE — a client opens one and
    /// a server ends one, and nothing in
    /// [`ClientFrame`](crate::frame::client::ClientFrame) says stop at
    /// that level. What a watch has instead is a
    /// [`channel_request`](crate::endpoints::volumes::watch::client::channel_request)
    /// that means it, and this is what sends one.
    ///
    /// A watch that is never disconnected runs until the connection
    /// does, with the provider reporting a tree nobody reads.
    ///
    /// # Saying it twice is harmless
    ///
    /// The second one opens another channel and says the same thing, and
    /// a provider that has already finished the scope will not have the
    /// scope to route it to. Nothing here refuses it, because nothing
    /// here knows: the terminal state lives on the stream, which a
    /// caller may have dropped.
    ///
    /// # The answer is discarded
    ///
    /// Nothing answers on the channel this opens, so the channel is
    /// abandoned as soon as the frame is out. Its entry in the router
    /// lingers until the scope closes — which is the thing the
    /// disconnect is provoking.
    ///
    /// # It fails one way
    ///
    /// The write, and only the write. Encoding a disconnect is
    /// [`Infallible`](std::convert::Infallible) — it is one byte with
    /// nothing in it — so there is no second failure to report and no
    /// error type of its own to carry it.
    pub async fn disconnect(&self) -> Result<(), SendError> {
        let mut payload = Vec::new();
        channel_request::Frame
            .encode(&mut Writer::new(&mut payload))
            .unwrap_or_else(|error| match error {});
        self.handle
            .send_channel_request(self.scope, &payload)
            .await
            .map(|_| ())
    }
}
