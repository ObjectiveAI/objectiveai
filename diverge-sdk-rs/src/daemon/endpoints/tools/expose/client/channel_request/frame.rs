//! What a client's channel request frame carries on an expose scope.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// What a client asks the daemon for while an expose is open.
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
    /// Let the exposure go. Tag `0`.
    ///
    /// Carries nothing — the variant is bare; the daemon knows which
    /// exposure, since the channel is on its scope — and nothing comes
    /// back on this channel: the daemon sends no channel response and
    /// no channel response finish on it. What comes back is the scope's
    /// finish: the exposure holds the tool no more and its
    /// authorization admits nothing from then on, the daemon finishes
    /// the scope, and a tool held by nothing else stops. This is how an
    /// expose, which never ends on its own while the tool runs, is
    /// ended by the client. A cancel of an expose that has already
    /// finished, or is finishing, changes nothing; a second cancel
    /// changes nothing either.
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

/// An expose channel request that could not be read.
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
            FrameError::Empty => f.write_str("tools expose channel request frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown tools expose channel request tag {tag}"),
        }
    }
}

impl std::error::Error for FrameError {}
