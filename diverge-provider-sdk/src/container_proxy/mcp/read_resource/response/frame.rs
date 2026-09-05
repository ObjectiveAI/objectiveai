//! Frames the server sends on `/mcp/read-resource`.

use super::super::FrameError;
use crate::decode::Decode as _;
use crate::encode::{Encode, Writer};
use crate::shared::mcp;

/// A frame sent by the server — the provider, on the connection it
/// opened into the container.
///
/// The answering half: responses on a channel the container opened,
/// and the finish that ends it. Typed, because the path names the
/// exchange: what answers on this path is always
/// [`mcp::read_resource::response::Frame`].
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// Type `0`. The answer. One of these, then the finish.
    ChannelResponse {
        /// The channel of the container request being answered.
        channel: u8,
        /// The resource's contents, or the server's error.
        response: mcp::read_resource::response::Frame,
    },
    /// Type `1`. The answer is complete and the channel is closed.
    /// Nothing follows on it, and the number is free again.
    ///
    /// With no response preceding it, this states that the exchange
    /// could not be served.
    ChannelResponseFinish {
        /// The channel of the container request being answered.
        channel: u8,
    },
}

impl Encode for Frame {
    /// The response's own failure: it is JSON. The finish cannot fail.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::ChannelResponse { channel, response } => {
                out.extend_from_slice(&[0, *channel]);
                response.encode(out)
            }
            Frame::ChannelResponseFinish { channel } => {
                out.extend_from_slice(&[1, *channel]);
                Ok(())
            }
        }
    }
}

impl Frame {
    /// Decode one frame from a WebSocket message's binary payload.
    pub fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        let (r#type, channel, payload) = super::super::split_header(bytes)?;
        match r#type {
            0 => mcp::read_resource::response::Frame::decode(payload)
                .map(|response| Frame::ChannelResponse { channel, response })
                .map_err(FrameError::Response),
            1 => Ok(Frame::ChannelResponseFinish { channel }),
            other => Err(FrameError::UnknownType(other)),
        }
    }
}
