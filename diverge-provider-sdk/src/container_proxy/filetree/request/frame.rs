//! The frame the container sends on `/filetree`.

use super::FrameError;
use crate::decode::Decode as _;
use crate::encode::{Encode, Writer};
use crate::shared::filetree;

/// A frame sent by the container: one filetree event, the whole
/// message.
///
/// A newtype over the shared frame, because the wire adds nothing to
/// it — no header, no channel, no type. The first on a connection is
/// a [`Snapshot`](filetree::response::Frame::Snapshot); every one
/// after is a delta.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame(pub filetree::response::Frame);

impl Encode for Frame {
    /// postcard's own failure, as the shared frame's is.
    type Error = postcard::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), postcard::Error> {
        self.0.encode(out)
    }
}

impl Frame {
    /// Decode one frame from a WebSocket message's binary payload.
    pub fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        filetree::response::Frame::decode(bytes)
            .map(Frame)
            .map_err(FrameError)
    }
}
