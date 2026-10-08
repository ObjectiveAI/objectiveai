//! What a client's channel request frame carries for a listing.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// What a lister asks a provider for while a listing is open: the
/// one thing, that it is done.
///
/// | tag | asks for |
/// |-----|----------|
/// | `0` | [`Stop`](Self::Stop) |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Frame {
    /// End the listing. Tag `0`.
    ///
    /// Carries nothing — the variant is bare — and nothing comes back
    /// on this channel. What comes back is the end of the SCOPE, a
    /// [`ResponseFinish`](crate::wire::frame::server::ServerFrame::ResponseFinish),
    /// which already means nothing bearing this scope follows on any
    /// channel. The listing has no end of its own — an identity that
    /// runs nothing is still watched — so this, the lister's
    /// connection ending, and the provider's error are the only ends
    /// it has; this is the one that says the lister is finished
    /// rather than gone.
    Stop,
}

/// Tag for [`Frame::Stop`].
const STOP: u8 = 0;

impl Encode for Frame {
    /// Nothing can fail: the frame is one byte.
    type Error = std::convert::Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        match self {
            Frame::Stop => {
                out.extend_from_slice(&[STOP]);
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
            STOP => Ok(Frame::Stop),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A listing channel request that could not be read.
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
            FrameError::Empty => f.write_str("tools list_for channel request frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown tools list_for channel request tag {tag}"),
        }
    }
}

impl std::error::Error for FrameError {}
