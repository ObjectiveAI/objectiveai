//! What a server's response frame carries for a get.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::error::Error;

use crate::daemon::endpoints::agents::list::server::response::Agent;

/// A get's answer: the agent, no agent is the one named, or a failure.
///
/// A get is one question and one reply, so there is exactly one of
/// these per scope, before the finish that ends it. A payload leads
/// with one byte saying which — `0` for [`Found`](Self::Found), `1` for
/// [`NotFound`](Self::NotFound), `2` for [`Error`](Self::Error) — and
/// the rest is that variant's own JSON: one
/// [`Agent`](crate::daemon::endpoints::agents::list::server::response::Agent)
/// for the first, as a list reports it; nothing for the second; the
/// error for the third.
///
/// # One answer, one failure
///
/// [`NotFound`](Self::NotFound) is an ANSWER: the daemon looked, and no
/// agent of the caller's is the one named, and nothing is retried. An
/// [`Error`](Self::Error) is the absence of an answer: the daemon could
/// not look, for whatever reason it knows.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The agent, as the list reports it. Tag `0`.
    Found(Agent),
    /// No agent of the caller's is the one named. Tag `1`.
    NotFound,
    /// A failure. Tag `2`.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error)
    /// for why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Found`].
const FOUND: u8 = 0;

/// Tag for [`Frame::NotFound`].
const NOT_FOUND: u8 = 1;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 2;

/// A tag, then the variant's own JSON, if it has any.
impl Encode for Frame {
    /// The ordinary JSON failure. The bare answer cannot fail.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Found(item) => {
                out.extend_from_slice(&[FOUND]);
                serde_json::to_writer(out, item)
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
    /// Four ways to fail, and each names which half failed.
    type Error = FrameError;

    // Spelled out for the same reason as `encode` above.
    fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            FOUND => serde_json::from_slice(rest).map(Frame::Found).map_err(FrameError::Found),
            NOT_FOUND => Ok(Frame::NotFound),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A get response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's three.
    UnknownTag(u8),
    /// The agent did not parse.
    Found(serde_json::Error),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("agents get response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown agents get response frame tag {tag}"),
            FrameError::Found(error) => write!(f, "agents get agent did not parse: {error}"),
            FrameError::Error(error) => write!(f, "agents get error did not parse: {error}"),
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Found(error) | FrameError::Error(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
