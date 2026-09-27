//! What a server's response frame carries for a create.

use std::fmt;

use crate::shared::error::Error;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// A create's answer: the tool is created, the name is in use, or a
/// failure.
///
/// A create is one question and one reply, so there is exactly one of
/// these per scope, before the finish that ends it. A payload leads
/// with one byte saying which — `0` for [`Created`](Self::Created), `1` for [`InUse`](Self::InUse), `2` for [`Error`](Self::Error) —
/// and only the error carries anything after it.
///
/// # The name in use is not a failure
///
/// [`InUse`](Self::InUse) is an ANSWER: a tool of the caller's exists
/// under that name already, and the daemon created nothing — the
/// caller uses the tool it has, or chooses another name, and nothing
/// is retried. An [`Error`](Self::Error) is the absence of an answer:
/// the tool could not be created, for whatever reason the daemon
/// knows, and the name is as it was.
///
/// # Created carries nothing
///
/// The name is the caller's handle from now on, and the caller chose
/// it: there is no id to hand back, and a tool that is not yet
/// attached to any agent runs nowhere.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The tool exists under the name. Tag `0`.
    Created,
    /// A tool of the caller's exists under that name already;
    /// nothing was created. Tag `1`.
    InUse,
    /// A failure. Tag `2`.
    ///
    /// The tool does not exist and will not, and the name is
    /// unchanged. See
    /// [`shared::error::Error`](crate::shared::error::Error)
    /// for why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Created`].
const CREATED: u8 = 0;

/// Tag for [`Frame::InUse`].
const IN_USE: u8 = 1;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 2;

/// A tag, and — for the error alone — that variant's own JSON.
impl Encode for Frame {
    /// The ordinary JSON failure. The bare answers cannot fail.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Created => {
                out.extend_from_slice(&[CREATED]);
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
            CREATED => Ok(Frame::Created),
            IN_USE => Ok(Frame::InUse),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A create response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's 3.
    UnknownTag(u8),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("tools create response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown tools create response frame tag {tag}"),
            FrameError::Error(error) => write!(f, "tools create error did not parse: {error}"),
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
