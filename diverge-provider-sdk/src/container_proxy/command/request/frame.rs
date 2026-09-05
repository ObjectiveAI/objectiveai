//! The frame the container sends on `/command`.

use std::convert::Infallible;

use super::super::FrameError;
use crate::encode::{Encode, Writer};

/// A frame sent by the container.
///
/// ```text
/// [channel: u8][command…]
/// ```
///
/// A struct, and no type byte in front of it, because the container
/// sends exactly one kind of frame: a command on a channel it
/// minted. The responses and the finish that answer this are the
/// server's — [`response::Frame`](super::super::response::Frame) — and the
/// container sends nothing on a channel it has opened.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Frame<'a> {
    /// A channel unique among the container's live channels, minted
    /// by the container.
    pub channel: u8,
    /// The command: bytes in the CLI's own vocabulary, which this
    /// layer never reads.
    pub command: &'a [u8],
}

impl Encode for Frame<'_> {
    /// [`Infallible`]: a channel byte and bytes copied.
    type Error = Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Infallible> {
        out.extend_from_slice(&[self.channel]);
        out.extend_from_slice(self.command);
        Ok(())
    }
}

impl<'a> Frame<'a> {
    /// Decode one frame from a WebSocket message's binary payload.
    /// The command borrows from `bytes`.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, FrameError> {
        let (channel, command) =
            bytes.split_first().ok_or(FrameError::Truncated)?;
        Ok(Frame {
            channel: *channel,
            command,
        })
    }
}
