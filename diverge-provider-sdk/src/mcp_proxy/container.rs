//! Frames the container sends.

use super::FrameError;
use crate::decode::Decode as _;
use crate::encode::{Encode, Writer};
use crate::endpoints::agentic_loop::run::server::channel_request;

/// A frame sent by the container.
///
/// One variant, because the container does one thing on this wire:
/// ask. The responses and the finish that answer it are the server's —
/// [`server::Frame`](super::server::Frame) — and the container sends
/// nothing on a channel it has opened.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// Type `0`. A request, opening a channel.
    ///
    /// One of these per channel, and never a second — the channel
    /// carries exactly one exchange, and the server's finish is what
    /// returns the number to the pool.
    ChannelRequest {
        /// A channel unique among the container's live channels,
        /// minted by the container — see [the module doc](super) for
        /// the discipline.
        channel: u8,
        /// The request: one MCP exchange, the same type a server's
        /// own channel request carries on the main wire. Typed rather
        /// than bytes, because there is exactly one thing this frame
        /// can hold and a second vocabulary for it would be a second
        /// place to be wrong.
        request: channel_request::Frame,
    },
}

/// A frame writes its own header, and the request writes itself
/// behind it — the same split the main protocol's frames make.
impl Encode for Frame {
    /// The request's own failure: its params are JSON.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        let Frame::ChannelRequest { channel, request } = self;
        out.extend_from_slice(&[0, *channel]);
        request.encode(out)
    }
}

impl Frame {
    /// Decode one frame from a WebSocket message's binary payload.
    pub fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        let (r#type, channel, payload) = super::split_header(bytes)?;
        match r#type {
            0 => Ok(Frame::ChannelRequest {
                channel,
                request: channel_request::Frame::decode(payload)
                    .map_err(FrameError::Request)?,
            }),
            other => Err(FrameError::UnknownType(other)),
        }
    }
}
