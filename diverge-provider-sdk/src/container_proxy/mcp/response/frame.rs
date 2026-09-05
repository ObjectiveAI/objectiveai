//! Frames the server sends on `/mcp`.

use std::convert::Infallible;

use super::super::FrameError;
use crate::encode::{Encode, Writer};

/// A frame sent by the server — the provider, on the connection it
/// opened into the container.
///
/// The answering half: responses on a channel the container opened,
/// and the finish that ends it. The server opens nothing — the
/// container is the only minter on this path.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame<'a> {
    /// Type `0`. One piece of the answer. There may be any number,
    /// including none.
    ///
    /// Bytes to this layer: which exchange it answers is known
    /// only to whoever opened the channel, and the opener decodes it
    /// with the response type of that exchange, as
    /// [`shared::mcp`](crate::shared::mcp) defines them.
    ChannelResponse {
        /// The channel of the container request being answered.
        channel: u8,
        /// The response bytes.
        payload: &'a [u8],
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

/// A frame writes its own header, and a payload never does.
impl Encode for Frame<'_> {
    /// [`Infallible`]: a header is fixed bytes and every payload is
    /// bytes already.
    type Error = Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Infallible> {
        match self {
            Frame::ChannelResponse { channel, payload } => {
                out.extend_from_slice(&[0, *channel]);
                out.extend_from_slice(payload);
            }
            Frame::ChannelResponseFinish { channel } => {
                out.extend_from_slice(&[1, *channel]);
            }
        }
        Ok(())
    }
}

impl<'a> Frame<'a> {
    /// Decode one frame from a WebSocket message's binary payload.
    /// The payload borrows from `bytes`.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, FrameError> {
        let (r#type, channel, payload) = super::super::split_header(bytes)?;
        match r#type {
            0 => Ok(Frame::ChannelResponse { channel, payload }),
            1 => Ok(Frame::ChannelResponseFinish { channel }),
            other => Err(FrameError::UnknownType(other)),
        }
    }
}
