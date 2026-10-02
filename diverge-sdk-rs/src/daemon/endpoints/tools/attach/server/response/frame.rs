//! What a server's response frame carries for an attach.

use std::fmt;

use crate::shared::error::Error;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// An attach's answer: the tool is attached to the agent, no tool or
/// no agent is the one named, or a failure.
///
/// An attach is one question and one reply, so there is exactly one of
/// these per scope, before the finish that ends it. A payload leads
/// with one byte saying which — `0` for [`Attached`](Self::Attached), `1` for [`NoTool`](Self::NoTool), `2` for [`NoAgent`](Self::NoAgent), `3` for [`Error`](Self::Error) —
/// and only the error carries anything after it.
///
/// # Two answers, one failure
///
/// [`NoTool`](Self::NoTool) and [`NoAgent`](Self::NoAgent) are
/// ANSWERS: the daemon looked, and one of the two names is nobody's,
/// so nothing was changed and nothing is retried — the caller has a
/// wrong name. An [`Error`](Self::Error) is the absence of an answer:
/// the daemon could not attach the tool, for whatever reason it
/// knows, and the attachment is as it was.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The tool is attached to the agent. Tag `0`.
    Attached,
    /// No tool of the caller's has the tool's name; nothing was
    /// changed. Tag `1`.
    NoTool,
    /// No agent of the caller's has the agent's name; nothing was
    /// changed. Tag `2`.
    NoAgent,
    /// A failure. Tag `3`.
    ///
    /// The tool is attached to the agent if it was, and not if it
    /// was not. See
    /// [`shared::error::Error`](crate::shared::error::Error)
    /// for why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Attached`].
const ATTACHED: u8 = 0;

/// Tag for [`Frame::NoTool`].
const NO_TOOL: u8 = 1;

/// Tag for [`Frame::NoAgent`].
const NO_AGENT: u8 = 2;

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
            Frame::Attached => {
                out.extend_from_slice(&[ATTACHED]);
                Ok(())
            }
            Frame::NoTool => {
                out.extend_from_slice(&[NO_TOOL]);
                Ok(())
            }
            Frame::NoAgent => {
                out.extend_from_slice(&[NO_AGENT]);
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
            ATTACHED => Ok(Frame::Attached),
            NO_TOOL => Ok(Frame::NoTool),
            NO_AGENT => Ok(Frame::NoAgent),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// An attach response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's 4.
    UnknownTag(u8),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("tools attach response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown tools attach response frame tag {tag}"),
            FrameError::Error(error) => write!(f, "tools attach error did not parse: {error}"),
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
