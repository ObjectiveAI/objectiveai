//! What a `secrets` channel is answered with.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::error::Error;

/// Fed, or not.
///
/// The whole answer to the [`Values`](super::super::super::Values) a
/// `secrets` channel opened with, and one frame is all there is —
/// this is not a stream, and a channel carrying one of these finishes
/// immediately after.
///
/// # The layout
///
/// ```text
/// [0]                  fed: the program holds the values
/// [1][error JSON …]    not fed, and why
/// ```
///
/// A feed says nothing else: the program took the values, and what it
/// does with them is its own. A refusal carries the far end's own
/// words — the proxy's, or the program's through it — and is the
/// activation's error: nothing is delivered to a program that was not
/// fed.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The program holds the values. Tag `0`.
    Fed,
    /// It does not, and this says why. Tag `1`. See
    /// [`shared::error::Error`](crate::shared::error::Error) for why
    /// it says so little.
    Error(Error),
}

/// Tag for [`Frame::Fed`].
const FED: u8 = 0;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 1;

impl Encode for Frame {
    /// The ordinary JSON failure, from the error alone.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Fed => {
                out.extend_from_slice(&[FED]);
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
    /// Three ways to fail, and only one of them is JSON.
    type Error = FrameError;

    // Spelled out for the same reason as `encode` above.
    fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            FED => Ok(Frame::Fed),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A feed answer that could not be read.
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
            FrameError::Empty => f.write_str("secrets feed answer frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown secrets feed answer frame tag {tag}"),
            FrameError::Error(error) => write!(f, "secrets feed error did not parse: {error}"),
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
