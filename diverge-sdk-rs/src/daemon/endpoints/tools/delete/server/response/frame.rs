//! What a server's response frame carries for a delete.

use std::fmt;

use crate::shared::error::Error;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// A delete's answer: the tool is deleted, no tool is the one named,
/// the tool is attached, or a failure.
///
/// A delete is one question and one reply, so there is exactly one of
/// these per scope, before the finish that ends it. A payload leads
/// with one byte saying which — `0` for [`Deleted`](Self::Deleted), `1`
/// for [`NotFound`](Self::NotFound), `2` for
/// [`Attached`](Self::Attached), `3` for
/// [`Forbidden`](Self::Forbidden), `4` for [`Error`](Self::Error) — and
/// only the error carries anything after it.
///
/// # Three answers, one failure
///
/// [`NotFound`](Self::NotFound) and [`Attached`](Self::Attached) are
/// ANSWERS: the daemon looked, and either no tool of the caller's has
/// the name, or one does and an agent has it attached, and in either
/// case nothing was deleted and nothing is retried — the caller has
/// the wrong name, or detaches the tool and asks again. An
/// [`Error`](Self::Error) is the absence of an answer: the daemon
/// could not delete the tool, for whatever reason it knows, and the
/// tool is as it was.
///
/// # Forbidden
///
/// [`Forbidden`](Self::Forbidden) is an answer every endpoint has: the
/// account the request is served for — the connection's, or the
/// `account` of the container it came from — holds no grant allowing
/// what the request asks over what it names, and nothing changed. Where
/// the answer is a stream it comes as an error does: exactly one, with
/// nothing before it, then the finish. Nothing is retried. See
/// [`grant`](crate::daemon::grant).
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The tool is deleted: its name free. Tag `0`.
    Deleted,
    /// No tool of the caller's is the one named; nothing was deleted.
    /// Tag `1`.
    NotFound,
    /// The tool is attached to at least one agent, and was left as
    /// it is; nothing was deleted. Tag `2`.
    Attached,
    /// The account the request is served for holds no grant allowing
    /// it; nothing changed. Tag `3`.
    Forbidden,
    /// A failure. Tag `4`.
    ///
    /// The tool is as it was. See
    /// [`shared::error::Error`](crate::shared::error::Error)
    /// for why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Deleted`].
const DELETED: u8 = 0;

/// Tag for [`Frame::NotFound`].
const NOT_FOUND: u8 = 1;

/// Tag for [`Frame::Attached`].
const ATTACHED: u8 = 2;

/// Tag for [`Frame::Forbidden`].
const FORBIDDEN: u8 = 3;

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
            Frame::Deleted => {
                out.extend_from_slice(&[DELETED]);
                Ok(())
            }
            Frame::NotFound => {
                out.extend_from_slice(&[NOT_FOUND]);
                Ok(())
            }
            Frame::Attached => {
                out.extend_from_slice(&[ATTACHED]);
                Ok(())
            }
            Frame::Forbidden => {
                out.extend_from_slice(&[FORBIDDEN]);
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
            ATTACHED => Ok(Frame::Attached),
            FORBIDDEN => Ok(Frame::Forbidden),
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
    /// A tag that is none of this frame's five.
    UnknownTag(u8),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("tools delete response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown tools delete response frame tag {tag}"),
            FrameError::Error(error) => write!(f, "tools delete error did not parse: {error}"),
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
