//! What an agentic loop channel carries back.

use std::fmt;

use super::AgenticLoopChunk;
use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::shared::error::Error;

/// A piece of the loop, or the news that there will not be one.
///
/// A payload leads with one byte saying which — `0` for
/// [`Chunk`](Self::Chunk), `1` for [`Error`](Self::Error) — and the
/// rest is that variant's own JSON.
///
/// # The tag is not decoration
///
/// [`AgenticLoopChunk`] is untagged and tells its own variants apart
/// by a `type` constant inside each one, while an [`Error`] is an
/// arbitrary JSON value — including, legitimately, an object with a
/// `type` field. Leaving the two to be distinguished by their JSON
/// would mean a provider's error text could be read as a chunk, and
/// the failure would look like output.
///
/// # This is not [`NotificationChunk`](super::NotificationChunk)
///
/// They are both failures and they are not the same failure. A
/// [`NotificationChunk`](super::NotificationChunk) with
/// [`is_fatal`](super::NotificationChunk::is_fatal) set is part of the
/// loop's OUTPUT: it arrives as a [`Chunk`](Self::Chunk) like any
/// other, and exists because a loop can fail after producing output —
/// ending the stream silently would leave a caller unable to tell a
/// partial result from a complete one. An [`Error`](Self::Error) is
/// not part of the loop. It is what a provider sends when there is no
/// loop to report on — the request the image would not take, the
/// container gone — and it carries a bare JSON value because this
/// specification does not describe what providers can go wrong with.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// One chunk of the loop. Tag `0`.
    Chunk(AgenticLoopChunk),
    /// A failure. Tag `1`.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Chunk`].
const CHUNK: u8 = 0;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 1;

impl Encode for Frame {
    /// The ordinary JSON failure, from either half.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Chunk(chunk) => {
                out.extend_from_slice(&[CHUNK]);
                serde_json::to_writer(out, chunk)
            }
            Frame::Error(error) => {
                out.extend_from_slice(&[ERROR]);
                error.encode(out)
            }
        }
    }
}

impl Decode<'_> for Frame {
    /// Four ways to fail, and each names which half failed.
    type Error = FrameError;

    // Spelled out for the same reason as `encode` above.
    fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            CHUNK => serde_json::from_slice(rest)
                .map(Frame::Chunk)
                .map_err(FrameError::Chunk),
            ERROR => {
                Error::decode(rest).map(Frame::Error).map_err(FrameError::Error)
            }
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// An agentic loop frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is neither of this frame's two.
    UnknownTag(u8),
    /// The chunk did not parse.
    Chunk(serde_json::Error),
    /// The error did not parse.
    ///
    /// Which is its own small joke and its own real problem: a
    /// provider whose failure report is malformed has told a caller
    /// that something went wrong and nothing else.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("agentic loop frame is empty")
            }
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown agentic loop frame tag {tag}")
            }
            FrameError::Chunk(error) => {
                write!(f, "agentic loop chunk did not parse: {error}")
            }
            FrameError::Error(error) => {
                write!(f, "agentic loop error did not parse: {error}")
            }
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Chunk(error) | FrameError::Error(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
