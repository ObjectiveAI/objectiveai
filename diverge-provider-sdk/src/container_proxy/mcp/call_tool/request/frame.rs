//! The frame the container sends on `/mcp/call-tool`.

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
    /// Run one tool: the name and the arguments, as rmcp defines them.
/// What an argument means belongs to the tool, and nothing between
/// here and it looks.
    pub request: mcp::call_tool::request::Request,
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
            request: mcp::call_tool::request::Request::decode(request)
                .map_err(FrameError::Request)?,
        })
    }
}
