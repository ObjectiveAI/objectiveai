//! One channel a provider opened inside a scope, and what comes back.

use bytes::Bytes;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use super::notice::Notice;

/// A channel this end opened inside a scope, and what comes back on it.
///
/// What
/// [`ScopeHandle::send_channel_request`](super::scope_handle::ScopeHandle::send_channel_request)
/// gives back. The request has gone out, and the session is holding the
/// other end of [`response_receiver`](Self::response_receiver).
///
/// The number is this end's. The client counts its own channels
/// separately and from zero, so a client's channel `1` and this one are
/// unrelated — which is why they never share a stream.
///
/// # Read it or drop it
///
/// [`response_receiver`](Self::response_receiver) is unbounded, and what rides it is a
/// stream rather than a message — an image layer, a database
/// connection, a command's items. An answer nobody reads is memory the
/// far side can grow without limit, and nothing in this crate bounds
/// it. Dropping this frees the queue and tells the session to forget
/// the channel.
#[derive(Debug)]
pub struct Channel {
    /// The channel's number, chosen by this end.
    ///
    /// Meaningful only inside the scope it was opened in.
    pub channel: u32,
    /// The client's answer, frame by frame.
    ///
    /// Whole frames, headers included. Ends at the finish frame; the
    /// channel closing without one means the connection went first, or
    /// the scope did.
    pub response_receiver: UnboundedReceiver<Bytes>,
    /// The scope it belongs to, for the notice at the end.
    ///
    /// Not public, because it is not this type's to tell — a caller
    /// that wants the scope's number has the
    /// [`ScopeHandle`](super::scope_handle::ScopeHandle) it came from.
    pub(super) scope: u32,
    /// Where to say this channel is over.
    pub(super) notice_sender: UnboundedSender<Notice>,
}

/// Tell the session the channel is over.
///
/// The same shape as
/// [`ScopeHandle`](super::scope_handle::ScopeHandle)'s, one level down
/// and for the same reason: close first, then say so, so that the
/// session's check reads as closed the moment the notice is visible.
///
/// A channel whose answer finished has already been forgotten — the
/// session drops the entry when it forwards the finish frame — so this
/// is for the other case, a caller that walked away mid-answer.
impl Drop for Channel {
    fn drop(&mut self) {
        self.response_receiver.close();
        let _ = self
            .notice_sender
            .send(Notice::Closed(self.scope, Some(self.channel)));
    }
}
