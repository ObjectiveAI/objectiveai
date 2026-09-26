//! What a caller answers on a tools channel.

use std::fmt;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::shared::error::Error;

/// Deployed, or not.
///
/// The whole answer to a [`Request`](super::super::request::Request),
/// and one frame is all there is — this is not a stream, and a
/// channel carrying one of these finishes immediately after.
///
/// # The layout
///
/// ```text
/// [0]                  deployed: every tool runs
/// [1][error JSON …]    not deployed, and why
/// ```
///
/// A deploy says nothing else: the provider needs no id of a tool
/// container, since the caller wires them, and nothing else acts on
/// the answer. A refusal carries the caller's own words, which the
/// run's error carries on.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// Every tool runs. Tag `0`.
    Deployed,
    /// A tool did not, and this says why. Tag `1`. See
    /// [`shared::error::Error`](crate::shared::error::Error) for why
    /// it says so little.
    Error(Error),
}

/// Tag for [`Frame::Deployed`].
const DEPLOYED: u8 = 0;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 1;

impl Encode for Frame {
    /// The ordinary JSON failure, from the error alone.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Deployed => {
                out.extend_from_slice(&[DEPLOYED]);
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
            DEPLOYED => Ok(Frame::Deployed),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A tools answer that could not be read.
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
            FrameError::Empty => f.write_str("tools answer frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown tools answer frame tag {tag}"),
            FrameError::Error(error) => write!(f, "tools error did not parse: {error}"),
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
