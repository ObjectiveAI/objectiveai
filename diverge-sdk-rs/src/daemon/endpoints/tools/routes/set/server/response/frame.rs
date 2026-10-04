//! What a server's response frame carries for a route set.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::error::Error;

/// A route set's answer: the route is down, no tool is the one named,
/// the tool is of the wrong template, the position is routed already,
/// or a failure.
///
/// A route set is one question and one reply, so there is exactly one
/// of these per scope, before the finish that ends it. A payload leads
/// with one byte saying which — `0` for [`Set`](Self::Set), `1` for
/// [`NoTool`](Self::NoTool), `2` for [`Mismatch`](Self::Mismatch), `3`
/// for [`Exists`](Self::Exists), `4` for [`Error`](Self::Error) — and
/// only the error carries anything after it.
///
/// # Answers, and one failure
///
/// [`NoTool`](Self::NoTool), [`Mismatch`](Self::Mismatch) and
/// [`Exists`](Self::Exists) are ANSWERS: the daemon looked, and no tool
/// of the caller's is the one named, or the tool named is not made from
/// the position's last template — a connected tool, made from none of
/// the caller's, always is not — or the position has a route, and in
/// each case nothing changed and nothing is retried. An
/// [`Error`](Self::Error) is the absence of an answer: the daemon could
/// not put the route down, for whatever reason it knows.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The route is down: the position is served the tool. Tag `0`.
    Set,
    /// No tool of the caller's is the one named; nothing changed. Tag
    /// `1`.
    NoTool,
    /// The tool is not made from the position's last template; nothing
    /// changed. Tag `2`.
    Mismatch,
    /// The position has a route already; nothing changed. Tag `3`.
    Exists,
    /// A failure. Tag `4`.
    ///
    /// Nothing changed. See
    /// [`shared::error::Error`](crate::shared::error::Error) for why it
    /// says so little.
    Error(Error),
}

/// Tag for [`Frame::Set`].
const SET: u8 = 0;

/// Tag for [`Frame::NoTool`].
const NO_TOOL: u8 = 1;

/// Tag for [`Frame::Mismatch`].
const MISMATCH: u8 = 2;

/// Tag for [`Frame::Exists`].
const EXISTS: u8 = 3;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 4;

/// A tag, and — for the error alone — that variant's own JSON.
impl Encode for Frame {
    /// The ordinary JSON failure. The bare answers cannot fail.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Set => {
                out.extend_from_slice(&[SET]);
                Ok(())
            }
            Frame::NoTool => {
                out.extend_from_slice(&[NO_TOOL]);
                Ok(())
            }
            Frame::Mismatch => {
                out.extend_from_slice(&[MISMATCH]);
                Ok(())
            }
            Frame::Exists => {
                out.extend_from_slice(&[EXISTS]);
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
            SET => Ok(Frame::Set),
            NO_TOOL => Ok(Frame::NoTool),
            MISMATCH => Ok(Frame::Mismatch),
            EXISTS => Ok(Frame::Exists),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A route set response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's five.
    UnknownTag(u8),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("tools routes set response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown tools routes set response frame tag {tag}"),
            FrameError::Error(error) => write!(f, "tools routes set error did not parse: {error}"),
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
