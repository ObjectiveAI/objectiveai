//! One frame off a channel this end opened, read for what it is.

use bytes::Bytes;

use crate::frame::client::ClientFrame;

/// What one frame on a server-opened channel means.
pub(crate) enum Answer {
    /// A response: its payload, a refcounted view into the frame.
    Frame(Bytes),
    /// The finish: the channel is over.
    Finish,
}

/// Read a frame the session routed onto a server-opened channel.
///
/// `None` for a frame that is not a channel response at all — the
/// session routes by header, so nothing else should arrive, and one
/// that does is dropped rather than read as something it is not.
pub(crate) fn answer(bytes: &Bytes) -> Option<Answer> {
    match ClientFrame::decode(bytes) {
        Ok(ClientFrame::ChannelResponse { payload, .. }) => Some(Answer::Frame(bytes.slice_ref(payload))),
        Ok(ClientFrame::ChannelResponseFinish { .. }) => Some(Answer::Finish),
        _ => None,
    }
}
