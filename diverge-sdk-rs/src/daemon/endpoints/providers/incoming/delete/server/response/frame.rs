//! What a server's response frame carries for a delete.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::error::Error;

/// A delete's answer: the judge is gone, no judge is the one named, the
/// judge is in use, or a failure.
///
/// A delete is one question and one reply, so there is exactly one of
/// these per scope, before the finish that ends it. A payload leads
/// with one byte saying which — `0` for [`Deleted`](Self::Deleted), `1`
/// for [`NotFound`](Self::NotFound), `2` for [`InUse`](Self::InUse),
/// `3` for [`Error`](Self::Error) — and only the error carries anything
/// after it.
///
/// # Answers, and one failure
///
/// [`NotFound`](Self::NotFound) and [`InUse`](Self::InUse) are ANSWERS: the daemon looked, and either no judge of the caller's is the one named, or one is and a provider is connected through it now, and in either case nothing changed and nothing is retried. An [`Error`](Self::Error) is the absence of an answer: the daemon could not take it out, for whatever reason it knows.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The judge is gone. Tag `0`.
    Deleted,
    /// No judge of the caller's is the one named; nothing changed. Tag
    /// `1`.
    NotFound,
    /// A provider is connected through the judge now, which was left as
    /// it is; nothing changed. Tag `2`.
    InUse,
    /// A failure. Tag `3`.
    ///
    /// Nothing changed. See
    /// [`shared::error::Error`](crate::shared::error::Error) for why it
    /// says so little.
    Error(Error),
}

/// Tag for [`Frame::Deleted`].
const DELETED: u8 = 0;

/// Tag for [`Frame::NotFound`].
const NOT_FOUND: u8 = 1;

/// Tag for [`Frame::InUse`].
const IN_USE: u8 = 2;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 3;

/// A tag, and — for the error alone — that variant's own JSON.
impl Encode for Frame {
    /// The ordinary JSON failure. The bare answers cannot fail.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Deleted => {
                out.extend_from_slice(&[DELETED]);
                Ok(())
            }
            Frame::NotFound => {
                out.extend_from_slice(&[NOT_FOUND]);
                Ok(())
            }
            Frame::InUse => {
                out.extend_from_slice(&[IN_USE]);
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
            DELETED => Ok(Frame::Deleted),
            NOT_FOUND => Ok(Frame::NotFound),
            IN_USE => Ok(Frame::InUse),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A delete response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's four.
    UnknownTag(u8),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("providers incoming delete response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown providers incoming delete response frame tag {tag}"),
            FrameError::Error(error) => write!(f, "providers incoming delete error did not parse: {error}"),
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
