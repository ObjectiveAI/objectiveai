//! What a server's response frame carries for an add.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::error::Error;

/// An add's answer: the judge is added, one like it exists, the hook's
/// resource is not there or not a hook, or a failure.
///
/// An add is one question and one reply, so there is exactly one of
/// these per scope, before the finish that ends it. A payload leads
/// with one byte saying which — `0` for [`Added`](Self::Added), `1` for
/// [`Exists`](Self::Exists), `2` for [`NoResource`](Self::NoResource),
/// `3` for [`Error`](Self::Error) — and only the error carries anything
/// after it.
///
/// # Answers, and one failure
///
/// [`Exists`](Self::Exists) and [`NoResource`](Self::NoResource) are ANSWERS: the daemon looked, and either a key judge for that identity or a hook judge for that resource is there already, or the hook names a resource the caller does not hold, or a file, or a directory with no `hook.yaml` at its root, and in each case nothing changed and nothing is retried. An [`Error`](Self::Error) is the absence of an answer: the daemon could not add it, for whatever reason it knows.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The judge is added, last in the order. Tag `0`.
    Added,
    /// A key judge for that identity, or a hook judge for that
    /// resource, is there already; nothing changed. Tag `1`.
    Exists,
    /// The hook names no resource the caller holds, or one that is not
    /// a directory with `hook.yaml` at its root; nothing changed. Tag
    /// `2`.
    NoResource,
    /// A failure. Tag `3`.
    ///
    /// Nothing changed. See
    /// [`shared::error::Error`](crate::shared::error::Error) for why it
    /// says so little.
    Error(Error),
}

/// Tag for [`Frame::Added`].
const ADDED: u8 = 0;

/// Tag for [`Frame::Exists`].
const EXISTS: u8 = 1;

/// Tag for [`Frame::NoResource`].
const NO_RESOURCE: u8 = 2;

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
            Frame::Added => {
                out.extend_from_slice(&[ADDED]);
                Ok(())
            }
            Frame::Exists => {
                out.extend_from_slice(&[EXISTS]);
                Ok(())
            }
            Frame::NoResource => {
                out.extend_from_slice(&[NO_RESOURCE]);
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
            ADDED => Ok(Frame::Added),
            EXISTS => Ok(Frame::Exists),
            NO_RESOURCE => Ok(Frame::NoResource),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// An add response frame that could not be read.
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
            FrameError::Empty => f.write_str("providers incoming add response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown providers incoming add response frame tag {tag}"),
            FrameError::Error(error) => write!(f, "providers incoming add error did not parse: {error}"),
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
