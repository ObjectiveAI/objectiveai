//! What arrives on a notification channel.

use rmcp::ErrorData;
use rmcp::model::ServerNotification;

use super::super::super::FrameError;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// One thing the server said on its own account, or the reason it will
/// say nothing further.
///
/// The payload of a
/// [`ClientFrame::ChannelResponse`](crate::wire::frame::client::ClientFrame::ChannelResponse)
/// on a channel opened by a
/// [`request::Request`](super::super::request::Request).
///
/// A payload leads with one byte saying which — `0` for
/// [`Notification`](Self::Notification), `1` for
/// [`Error`](Self::Error) — and the rest is that variant's own JSON.
///
/// # Many frames, unlike the other four
///
/// The other exchanges answer once and finish. This one carries a frame
/// per notification for as long as the channel lives, because that is
/// what it is: not an answer, but the stream a server pushes into when
/// something changes.
///
/// Nothing terminates it in the payload. The channel's own finish says
/// there will be no more, which is what a finish already means and what
/// makes a second signal for one fact a second thing to disagree about.
///
/// # An error is the last thing on it
///
/// A notification stream that stops is a stream that stopped, and a
/// caller that can no longer keep one open says so here rather than
/// finishing in silence. Nothing follows it; the channel finishes
/// after.
///
/// It is [`ErrorData`] rather than
/// [`shared::error::Error`](crate::shared::error::Error) for the reason
/// the other four give: what is being relayed is an MCP server's
/// failure, not the provider's, and a JSON-RPC code is content rather
/// than detail.
///
/// # [`ServerNotification`] is the whole union, deliberately
///
/// Every notification a server can send, including the ones this crate
/// has no opinion about. Narrowing it to the ones an agent obviously
/// acts on — tools changed, resources changed — would mean deciding on
/// an agent's behalf what is worth hearing, and would need revising
/// every time MCP adds one.
///
/// It deserializes by its `method`, which each variant fixes to a
/// constant that rejects every other value. So the union is untagged in
/// serde's terms and discriminated in practice, and a notification this
/// crate's `rmcp` is too old to know arrives as
/// `CustomNotification` rather than as a failure.
///
/// # It does not derive [`PartialEq`]
///
/// Alone among the five, because
/// [`ServerNotification`] does not. Comparing two is not something
/// anything here does, and wrapping the union to add it would be
/// keeping a second copy of `rmcp`'s type for the sake of a derive.
#[derive(Debug, Clone)]
pub enum Frame {
    /// One notification, as the server sent it. Tag `0`.
    Notification(ServerNotification),
    /// The stream ended early, and this is why. Tag `1`.
    Error(ErrorData),
}

/// Tag for [`Frame::Notification`].
const NOTIFICATION: u8 = 0;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 1;

impl Encode for Frame {
    /// The ordinary JSON failure. Both variants are serialized and the
    /// tag cannot fail.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Notification(notification) => {
                out.extend_from_slice(&[NOTIFICATION]);
                serde_json::to_writer(out, notification)
            }
            Frame::Error(error) => {
                out.extend_from_slice(&[ERROR]);
                serde_json::to_writer(out, error)
            }
        }
    }
}

impl Decode<'_> for Frame {
    /// Three ways to fail, and only one of them is JSON.
    type Error = FrameError;

    // Spelled out for the same reason as `encode` above.
    fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            NOTIFICATION => serde_json::from_slice(rest)
                .map(Frame::Notification)
                .map_err(FrameError::Body),
            ERROR => serde_json::from_slice(rest)
                .map(Frame::Error)
                .map_err(FrameError::Body),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}
