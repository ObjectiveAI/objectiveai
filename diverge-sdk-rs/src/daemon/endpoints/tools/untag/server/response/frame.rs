//! What a server's response frame carries for an untag.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::error::Error;

/// An untag's answer: the tags are off the tool, no tool has
/// the name, or a failure.
///
/// An untag is one question and one reply, so there is exactly one of
/// these per scope, before the finish that ends it. A payload leads
/// with one byte saying which — `0` for [`Untagged`](Self::Untagged),
/// `1` for [`NotFound`](Self::NotFound), `2` for
/// [`Error`](Self::Error) — and only the error carries anything
/// after it.
///
/// # One answer, one failure
///
/// [`NotFound`](Self::NotFound) is an ANSWER: the daemon looked, and
/// no tool of the caller's has the name, and nothing is retried —
/// the caller has the wrong name. An [`Error`](Self::Error) is the
/// absence of an answer: the daemon could not change the tool's
/// tags, for whatever reason it knows, and the tags are as they were.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The tags are off the tool. Tag `0`.
    Untagged,
    /// No tool of the caller's has the name; nothing changed.
    /// Tag `1`.
    NotFound,
    /// A failure. Tag `2`.
    ///
    /// The tags are as they were. See
    /// [`shared::error::Error`](crate::shared::error::Error)
    /// for why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Untagged`].
const UNTAGGED: u8 = 0;

/// Tag for [`Frame::NotFound`].
const NOT_FOUND: u8 = 1;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 2;

/// A tag, and — for the error alone — that variant's own JSON.
impl Encode for Frame {
    /// The ordinary JSON failure. The two bare answers cannot fail.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Untagged => {
                out.extend_from_slice(&[UNTAGGED]);
                Ok(())
            }
            Frame::NotFound => {
                out.extend_from_slice(&[NOT_FOUND]);
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
            UNTAGGED => Ok(Frame::Untagged),
            NOT_FOUND => Ok(Frame::NotFound),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// An untag response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's three.
    UnknownTag(u8),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("tools untag response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown tools untag response frame tag {tag}"),
            FrameError::Error(error) => write!(f, "tools untag error did not parse: {error}"),
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
