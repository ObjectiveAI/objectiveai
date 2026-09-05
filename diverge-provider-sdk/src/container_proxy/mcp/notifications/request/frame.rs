//! The frame the container sends on `/mcp/notifications`.

use std::convert::Infallible;

use super::super::FrameError;
use crate::decode::Decode as _;
use crate::encode::{Encode, Writer};
use crate::shared::mcp;

/// A frame sent by the container.
///
/// ```text
/// [channel: u8][params JSON…]
/// ```
///
/// A struct, and no type byte in front of it, because the container
/// sends exactly one kind of frame on this path: this exchange, on
/// a channel it minted. The responses and the finish that answer it
/// are the server's — [`response::Frame`](super::super::response::Frame)
/// — and the container sends nothing on a channel it has opened.
///
/// One of these per channel, and never a second — the channel carries
/// exactly one exchange, and the server's finish is what returns the
/// number to the pool.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    /// A channel unique among the container's live channels, minted
    /// by the container.
    pub channel: u8,
    /// Everything the servers say on their own account: tools changed,
/// resources changed, a resource updated, a log line. It carries
/// nothing — in Streamable HTTP a client opens the notification
/// stream with a bare `GET`, no method, no body — so the empty
/// payload is not an economy. It is the request, whole.
    pub request: mcp::notifications::request::Request,
}

impl Encode for Frame {
    /// [`Infallible`]: a channel byte, and a request that carries
    /// nothing.
    type Error = Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Infallible> {
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
            request: mcp::notifications::request::Request::decode(request)
                .unwrap_or_else(|error| match error {}),
        })
    }
}
