//! The frame the server sends on `/mcp/notifications`.

use super::super::FrameError;
use crate::decode::Decode as _;
use crate::encode::{Encode, Writer};
use crate::shared::mcp;

/// A frame sent by the server: one notification, the whole message.
///
/// A newtype over the shared frame, because the wire adds nothing to
/// it — no header, no channel, no type. An
/// [`Error`](mcp::notifications::response::Frame::Error) is the last
/// one on a connection.
///
/// No `PartialEq`, because the notification it carries has none.
#[derive(Debug, Clone)]
pub struct Frame(pub mcp::notifications::response::Frame);

impl Encode for Frame {
    /// The notification's own failure: it is JSON.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        self.0.encode(out)
    }
}

impl Frame {
    /// Decode one frame from a WebSocket message's binary payload.
    pub fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        mcp::notifications::response::Frame::decode(bytes)
            .map(Frame)
            .map_err(FrameError)
    }
}
