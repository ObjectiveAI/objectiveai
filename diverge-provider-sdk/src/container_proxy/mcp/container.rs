//! The frame the container sends on `/mcp`.

use super::FrameError;
use super::request::Request;
use crate::encode::{Encode, Writer};

/// A frame sent by the container.
///
/// ```text
/// [channel: u8][request…]
/// ```
///
/// A struct, and no type byte in front of it, because the container
/// sends exactly one kind of frame: an exchange on a channel it
/// minted. Which exchange is inside the request. The responses and
/// the finish that answer this are the server's —
/// [`server::Frame`](super::server::Frame) — and the container sends
/// nothing on a channel it has opened.
///
/// One of these per channel, and never a second — the channel carries
/// exactly one exchange, and the server's finish is what returns the
/// number to the pool.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    /// A channel unique among the container's live channels, minted
    /// by the container.
    pub channel: u8,
    /// The exchange asked for.
    pub request: Request,
}

impl Encode for Frame {
    /// The request's own failure: its params are JSON.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        out.extend_from_slice(&[self.channel]);
        self.request.encode(out)
    }
}

impl Frame {
    /// Decode one frame from a WebSocket message's binary payload.
    pub fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        let (channel, request) =
            bytes.split_first().ok_or(FrameError::Truncated)?;
        Ok(Frame {
            channel: *channel,
            request: Request::decode(request).map_err(FrameError::Request)?,
        })
    }
}
