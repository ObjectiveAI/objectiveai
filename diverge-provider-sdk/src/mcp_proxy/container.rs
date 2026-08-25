//! The frame the container sends.

use super::FrameError;
use crate::decode::Decode as _;
use crate::encode::{Encode, Writer};
use crate::endpoints::agentic_loop::run::server::channel_request;

/// A frame sent by the container.
///
/// ```text
/// [channel: u8][request…]
/// ```
///
/// A struct, and no type byte in front of it, because the container
/// sends exactly one kind of frame: an enum of one variant is a
/// discriminant with nothing to discriminate, and a byte that says
/// the only thing it could say is a byte not spent. The responses and
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
    /// by the container — see [the module doc](super) for the
    /// discipline.
    pub channel: u8,
    /// The request: one MCP exchange, the same type a server's own
    /// channel request carries on the main wire. Typed rather than
    /// bytes, because there is exactly one thing this frame can hold
    /// and a second vocabulary for it would be a second place to be
    /// wrong.
    pub request: channel_request::Frame,
}

/// A frame writes its own header, and the request writes itself
/// behind it — the same split the main protocol's frames make.
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
            request: channel_request::Frame::decode(request)
                .map_err(FrameError::Request)?,
        })
    }
}
