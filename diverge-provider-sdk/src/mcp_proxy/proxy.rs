//! Frames the proxy sends.

use std::convert::Infallible;

use super::FrameError;
use crate::encode::{Encode, Writer};

/// A frame sent by the proxy.
///
/// One variant, because the proxy does one thing: ask. The responses
/// and the finish that answer it are the provider's —
/// [`provider::Frame`](super::provider::Frame) — and the proxy sends
/// nothing on a channel it has opened.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame<'a> {
    /// Type `0`. A request, opening a channel.
    ///
    /// One of these per channel, and never a second — the channel
    /// carries exactly one exchange, and the provider's finish is
    /// what returns the number to the pool.
    ChannelRequest {
        /// A channel unique among the proxy's live channels, minted
        /// by the proxy — see [the module doc](super) for the
        /// discipline.
        channel: u8,
        /// The request bytes: the exchange tag and its params, as the
        /// endpoint layer defines them.
        payload: &'a [u8],
    },
}

/// A frame writes its own header, and a payload never does — the same
/// split the main protocol's frames make, for the same reason.
impl Encode for Frame<'_> {
    /// [`Infallible`]: a header is fixed bytes and the payload is
    /// bytes already.
    type Error = Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Infallible> {
        let Frame::ChannelRequest { channel, payload } = self;
        out.extend_from_slice(&[0, *channel]);
        out.extend_from_slice(payload);
        Ok(())
    }
}

impl<'a> Frame<'a> {
    /// Decode one frame from a WebSocket message's binary payload.
    /// The payload borrows from `bytes`.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, FrameError> {
        let (r#type, channel, payload) = super::split_header(bytes)?;
        match r#type {
            0 => Ok(Frame::ChannelRequest { channel, payload }),
            other => Err(FrameError::UnknownType(other)),
        }
    }
}
