//! The frame the container sends on `/vault`.

use super::FrameError;
use super::request::{Request, RequestEncodeError};
use crate::encode::{Encode, Writer};

/// A frame sent by the container.
///
/// ```text
/// [channel: u8][request…]
/// ```
///
/// A struct, and no type byte in front of it, because the container
/// sends exactly one kind of frame: a request on a channel it
/// minted. What kind of request is inside the request. The response
/// and the finish that answer this are the server's —
/// [`server::Frame`](super::server::Frame) — and the container sends
/// nothing on a channel it has opened.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Frame<'a> {
    /// A channel unique among the container's live channels, minted
    /// by the container.
    pub channel: u8,
    /// The request.
    pub request: Request<'a>,
}

impl Encode for Frame<'_> {
    /// The request's own failure.
    type Error = RequestEncodeError;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), RequestEncodeError> {
        out.extend_from_slice(&[self.channel]);
        self.request.encode(out)
    }
}

impl<'a> Frame<'a> {
    /// Decode one frame from a WebSocket message's binary payload.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, FrameError> {
        let (channel, request) =
            bytes.split_first().ok_or(FrameError::Truncated)?;
        Ok(Frame {
            channel: *channel,
            request: Request::decode(request).map_err(FrameError::Request)?,
        })
    }
}
