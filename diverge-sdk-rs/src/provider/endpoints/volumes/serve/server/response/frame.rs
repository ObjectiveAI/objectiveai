//! What a server's response frame carries for a volume serve.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::error::Error;

/// The volume is served, or the news that it will not be.
///
/// A payload leads with one byte saying which — `0` for
/// [`Serving`](Self::Serving), `1` for [`Error`](Self::Error) — and
/// the rest is that variant's own bytes, of which `Serving` has none.
///
/// # How the scope runs
///
/// | the scope | means |
/// |-----------|-------|
/// | a serving, then quiet, and stays open | the volume is held, and every ask the caller opens is answered on its channel |
/// | a serving, then a finish | the caller stopped, and the volume is given back |
/// | an error, then a finish | it was not served: no volume of the caller's has the name, or it is held exclusively |
///
/// Exactly one response, first. Nothing else comes on channel `0`;
/// the answers to asks come on the asks' channels.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The volume is held, and asks may be opened. Tag `0`.
    Serving,
    /// A failure. Tag `1`.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Serving`].
const SERVING: u8 = 0;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 1;

impl Encode for Frame {
    /// The ordinary JSON failure, from the half that has one.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Serving => {
                out.extend_from_slice(&[SERVING]);
                Ok(())
            }
            Frame::Error(error) => {
                out.extend_from_slice(&[ERROR]);
                error.encode(out)
            }
        }
    }
}

impl Decode<'_> for Frame {
    /// Three ways to fail, and only one of them is a parse.
    type Error = FrameError;

    // Spelled out for the same reason as `encode` above.
    fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            SERVING => Ok(Frame::Serving),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A volume serve response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is neither of this frame's two.
    UnknownTag(u8),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("volume serve response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown volume serve response frame tag {tag}"),
            FrameError::Error(error) => write!(f, "volume serve error did not parse: {error}"),
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Error(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
