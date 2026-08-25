//! Frames the provider sends.

use std::convert::Infallible;

use super::FrameError;
use crate::encode::{Encode, Writer};

/// A frame sent by the provider.
///
/// The answering half: responses on a channel the proxy opened, and
/// the finish that ends it. The provider opens nothing — the proxy is
/// the only minter on this wire.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame<'a> {
    /// Type `1`. One piece of the answer. There may be any number,
    /// including none.
    ChannelResponse {
        /// The channel of the proxy request being answered.
        channel: u8,
        /// The response bytes: the exchange's response encoding —
        /// result or error — as
        /// [`shared::mcp`](crate::shared::mcp) defines it.
        payload: &'a [u8],
    },
    /// Type `2`. The answer is complete and the channel is closed.
    /// Nothing follows on it, and the number is free again.
    ///
    /// With no response preceding it, this states that the exchange
    /// could not be served — the standing convention, unchanged.
    ChannelResponseFinish {
        /// The channel of the proxy request being answered.
        channel: u8,
    },
}

/// A frame writes its own header, and a payload never does — the same
/// split the main protocol's frames make, for the same reason.
impl Encode for Frame<'_> {
    /// [`Infallible`]: a header is fixed bytes and every payload is
    /// bytes already.
    type Error = Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Infallible> {
        match self {
            Frame::ChannelResponse { channel, payload } => {
                out.extend_from_slice(&[1, *channel]);
                out.extend_from_slice(payload);
            }
            Frame::ChannelResponseFinish { channel } => {
                out.extend_from_slice(&[2, *channel]);
            }
        }
        Ok(())
    }
}

impl<'a> Frame<'a> {
    /// Decode one frame from a WebSocket message's binary payload.
    /// The payload borrows from `bytes`.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, FrameError> {
        let (r#type, channel, payload) = super::split_header(bytes)?;
        match r#type {
            1 => Ok(Frame::ChannelResponse { channel, payload }),
            2 => Ok(Frame::ChannelResponseFinish { channel }),
            other => Err(FrameError::UnknownType(other)),
        }
    }
}
