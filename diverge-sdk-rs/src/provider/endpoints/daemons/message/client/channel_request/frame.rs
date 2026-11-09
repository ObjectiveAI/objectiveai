//! What a client's channel request frame carries for a message.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// What a caller asks a provider for while a message is open: the one
/// thing, that it is done.
///
/// | tag | asks for |
/// |-----|----------|
/// | `0` | [`Cancel`](Self::Cancel) |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Frame {
    /// Take the message back. Tag `0`.
    ///
    /// Carries nothing — the variant is bare; the provider knows which
    /// message, since the channel is on its scope — and nothing comes
    /// back on this channel: the provider sends no channel response and
    /// no channel response finish on it. A cancel is in time while the
    /// message waits in the receiver's queue: the message is taken out,
    /// the receiver never sees it, and the scope finishes with no
    /// response before the finish. A cancel after the provider sent the
    /// message to the receiver changes nothing, and the response comes
    /// as it would have: the response, not the cancel, says which was
    /// first. A second cancel changes nothing either.
    Cancel,
}

/// Tag for [`Frame::Cancel`].
const CANCEL: u8 = 0;

impl Encode for Frame {
    /// Nothing can fail: the frame is one byte.
    type Error = std::convert::Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        match self {
            Frame::Cancel => {
                out.extend_from_slice(&[CANCEL]);
                Ok(())
            }
        }
    }
}

impl Decode<'_> for Frame {
    /// Two ways to fail: nothing, or a tag that is not the one.
    type Error = FrameError;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        let (tag, _) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            CANCEL => Ok(Frame::Cancel),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A message channel request that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is not this frame's one.
    UnknownTag(u8),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("message channel request frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown message channel request tag {tag}"),
        }
    }
}

impl std::error::Error for FrameError {}
