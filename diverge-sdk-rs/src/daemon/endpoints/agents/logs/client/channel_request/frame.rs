//! What a client's channel request frame carries on a logs scope.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// What a client asks the daemon for while a logs read is open.
///
/// A payload leads with one byte saying which, and the rest is that
/// variant's own bytes.
///
/// | tag | asks for |
/// |-----|----------|
/// | `0` | [`Cancel`](Self::Cancel) |
///
/// One, and it answers nothing: what it does is said by the scope
/// itself, which finishes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Frame {
    /// Stop reading. Tag `0`.
    ///
    /// Carries nothing — the variant is bare; the daemon knows which
    /// read, since the channel is on its scope — and nothing comes
    /// back on this channel: the daemon sends no channel response and
    /// no channel response finish on it. What comes back is the
    /// scope's finish: the daemon stops running the filter and
    /// finishes the scope, and values it had already sent precede
    /// the finish, since a cancel is a request and not a recall. This
    /// is how a watch that would never end on its own — one with no
    /// last id and no last time — is ended. A cancel of a read that
    /// has already finished, or is finishing, changes nothing; a
    /// second cancel changes nothing either.
    Cancel,
}

/// Tag for [`Frame::Cancel`].
const CANCEL: u8 = 0;

/// One byte, laid out by hand. One tag is not a shape a format would
/// help with.
impl Encode for Frame {
    /// [`Infallible`](std::convert::Infallible): a known byte.
    type Error = std::convert::Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        match self {
            Frame::Cancel => out.extend_from_slice(&[CANCEL]),
        }
        Ok(())
    }
}

impl Decode<'_> for Frame {
    /// Two ways to fail, and neither of them is a parse.
    type Error = FrameError;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        let (tag, _) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            CANCEL => Ok(Frame::Cancel),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A logs channel request that could not be read.
///
/// Anything after the tag is ignored rather than refused: nothing is
/// defined to follow one, and leaving room for it is cheaper than
/// rejecting a request whose whole meaning already arrived.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is not this frame's one.
    UnknownTag(u8),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("agents logs channel request frame is empty"),
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown agents logs channel request tag {tag}")
            }
        }
    }
}

impl std::error::Error for FrameError {}
