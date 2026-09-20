//! What a server's response frame carries for a delete.

use std::fmt;

use diverge_provider_sdk::decode::Decode;
use diverge_provider_sdk::encode::{Encode, Writer};
use diverge_provider_sdk::shared::error::Error;

/// A delete's answer: the agent is deleted, no agent has the name,
/// the agent is active, or a failure.
///
/// A delete is one question and one reply, so there is exactly one of
/// these per scope, before the finish that ends it. A payload leads
/// with one byte saying which — `0` for [`Deleted`](Self::Deleted),
/// `1` for [`NotFound`](Self::NotFound), `2` for
/// [`Active`](Self::Active), `3` for [`Error`](Self::Error) — and
/// only the error carries anything after it.
///
/// # Three answers, one failure
///
/// [`NotFound`](Self::NotFound) and [`Active`](Self::Active) are
/// ANSWERS: the daemon looked, and either no agent of the caller's
/// has the name, or one does and a loop is running in it, and in
/// either case nothing was deleted and nothing is retried — the
/// caller has the wrong name, or waits for the loop to end and asks
/// again. An [`Error`](Self::Error) is the absence of an answer: the
/// daemon could not delete the agent, for whatever reason it knows,
/// and the agent is as it was.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The agent is deleted: its container stopped, its name free.
    /// Tag `0`.
    Deleted,
    /// No agent of the caller's has the name; nothing was deleted.
    /// Tag `1`.
    NotFound,
    /// The agent is active — a loop is running in it — and was left
    /// as it is; nothing was deleted. Tag `2`.
    Active,
    /// A failure. Tag `3`.
    ///
    /// The agent is as it was. See
    /// [`shared::error::Error`](diverge_provider_sdk::shared::error::Error)
    /// for why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Deleted`].
const DELETED: u8 = 0;

/// Tag for [`Frame::NotFound`].
const NOT_FOUND: u8 = 1;

/// Tag for [`Frame::Active`].
const ACTIVE: u8 = 2;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 3;

/// A tag, and — for the error alone — that variant's own JSON.
impl Encode for Frame {
    /// The ordinary JSON failure. The three bare answers cannot fail.
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
            Frame::Active => {
                out.extend_from_slice(&[ACTIVE]);
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
            ACTIVE => Ok(Frame::Active),
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
            FrameError::Empty => f.write_str("agents delete response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown agents delete response frame tag {tag}"),
            FrameError::Error(error) => write!(f, "agents delete error did not parse: {error}"),
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
