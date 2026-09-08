//! The answer to a dequeue: whether the queue held anything.

use std::error;
use std::fmt;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::shared;

/// One frame, then the finish: what the clearing found.
///
/// A payload leads with one byte saying which; only
/// [`Error`](Self::Error) carries anything after it. The messages
/// themselves are not restated here — each withdrawn message's own
/// enqueue channel says
/// [`Dequeued`](crate::shared::containers::enqueue::response::Frame::Dequeued),
/// and this answer is only the clearing's summary.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The queue held messages, and they are withdrawn. Tag `0`.
    Dequeued,
    /// The queue held nothing. Tag `1`.
    ///
    /// Not a failure: everything previously enqueued had already
    /// been taken, withdrawn, or missed — or no loop was running —
    /// and there was nothing left for the clearing to do.
    Empty,
    /// The queue's state could not be determined. Tag `2`.
    ///
    /// The provider's own failure, in the protocol's one error
    /// shape.
    Error(shared::error::Error),
}

/// Tag for [`Frame::Dequeued`].
const DEQUEUED: u8 = 0;

/// Tag for [`Frame::Empty`].
const EMPTY: u8 = 1;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 2;

/// A tag, and — for the error alone — that variant's own JSON.
impl Encode for Frame {
    /// The ordinary JSON failure. Two variants cannot fail at all.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Dequeued => {
                out.extend_from_slice(&[DEQUEUED]);
                Ok(())
            }
            Frame::Empty => {
                out.extend_from_slice(&[EMPTY]);
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
            DEQUEUED => Ok(Frame::Dequeued),
            EMPTY => Ok(Frame::Empty),
            ERROR => shared::error::Error::decode(rest)
                .map(Frame::Error)
                .map_err(FrameError::Body),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A dequeue answer that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's three.
    ///
    /// What a provider newer than its caller produces, which is the
    /// case the tag exists to make survivable: a reader that does not
    /// know a variant says so, rather than reading somebody else's
    /// bytes as its own.
    UnknownTag(u8),
    /// The error payload after the tag did not parse.
    Body(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("dequeue answer frame is empty"),
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown dequeue answer tag {tag}")
            }
            FrameError::Body(error) => {
                write!(f, "dequeue answer error did not parse: {error}")
            }
        }
    }
}

impl error::Error for FrameError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            FrameError::Body(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
