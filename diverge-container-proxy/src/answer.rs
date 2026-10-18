//! What comes back on a channel the proxy opened, one frame at a time.

use bytes::Bytes;
use diverge_sdk::wire::frame::client::ClientFrame;
use diverge_sdk::wire::server::channel::Channel;

/// One thing the server said on a channel of the proxy's.
pub enum Answer {
    /// One message of the answer: a channel response's payload.
    Frame(Bytes),
    /// The finish: the answer is whole.
    Finish,
}

/// The next thing on the channel: a frame, the finish, or `None` when
/// the channel closed without one — the connection went first, or the
/// scope did, and the answer will never be whole.
///
/// The session routes only the channel's own responses here, whole
/// frames with their headers; a frame that is neither of the two is
/// passed over.
pub async fn next(channel: &mut Channel) -> Option<Answer> {
    loop {
        let bytes = channel.response_receiver.recv().await?;
        match ClientFrame::decode(&bytes) {
            Ok(ClientFrame::ChannelResponse { payload, .. }) => {
                return Some(Answer::Frame(bytes.slice_ref(payload)));
            }
            Ok(ClientFrame::ChannelResponseFinish { .. }) => return Some(Answer::Finish),
            _ => {}
        }
    }
}
