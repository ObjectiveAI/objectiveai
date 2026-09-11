//! One channel a caller opened inside a scope, and what comes back.

use bytes::Bytes;
use tokio::sync::mpsc::UnboundedReceiver;

/// A channel that has been opened inside a scope, and what comes back
/// on it.
///
/// What
/// [`Handle::send_channel_request`](super::handle::Handle::send_channel_request)
/// gives back. The request has gone out, and the router is holding the
/// other end of [`response_receiver`](Self::response_receiver).
///
/// The number is this end's. The server counts its own channels
/// separately and from zero, so a server's channel `1` and this one are
/// unrelated — which is why they never share a stream.
#[derive(Debug)]
pub struct Channel {
    /// The channel's number, chosen by this end.
    ///
    /// Meaningful only inside the scope it was opened in. Free for
    /// reuse once the channel closes.
    pub channel: u32,
    /// The server's answer, frame by frame.
    ///
    /// Whole frames, headers included. Ends at the finish frame; the
    /// channel closing without one means the connection went first, or
    /// the scope did.
    pub response_receiver: UnboundedReceiver<Bytes>,
}
