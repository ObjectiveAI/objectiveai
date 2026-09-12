//! The frame the container sends on `/requests`.

use super::{FrameError, Request, RequestEncodeError};
use crate::encode::{Encode, Writer};

/// One ask, and the channel its answer will come on.
///
/// ```text
/// [channel: u32 big-endian][kind: u8][payload…]
/// ```
///
/// The header is the channel alone; what kind of ask follows is the
/// [`Request`]'s own first byte. The server answers by opening the
/// ask's path with this channel in it — see [the
/// module](super::super::super).
#[derive(Debug, Clone, PartialEq)]
pub struct Frame<'a> {
    /// The channel, minted by the container: unique among its
    /// requests not yet answered, counted up.
    pub channel: u32,
    /// The ask.
    pub request: Request<'a>,
}

/// The bytes the channel occupies.
const CHANNEL_LEN: usize = 4;

impl Encode for Frame<'_> {
    /// The request's own failure.
    type Error = RequestEncodeError;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), RequestEncodeError> {
        out.extend_from_slice(&self.channel.to_be_bytes());
        self.request.encode(out)
    }
}

impl<'a> Frame<'a> {
    /// Decode one frame from a WebSocket message's binary payload.
    /// What the request borrows, it borrows from `bytes`.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, FrameError> {
        let channel: &[u8; CHANNEL_LEN] = bytes
            .get(..CHANNEL_LEN)
            .and_then(|head| head.try_into().ok())
            .ok_or(FrameError::Truncated)?;
        Ok(Frame {
            channel: u32::from_be_bytes(*channel),
            request: Request::decode(&bytes[CHANNEL_LEN..])?,
        })
    }
}
