//! What a server's response frame carries for list.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::error::Error;

use super::Agent;

/// A list's answer: one agent, forbidden, or a failure.
///
/// A list is a stream: zero or more agents, each one that matches as
/// the daemon holds it, oldest first, then the finish; or exactly one
/// [`Forbidden`](Self::Forbidden), then the finish; or agents and then
/// exactly one error, then the finish. A count on the request caps
/// them. A payload leads with one byte saying which — `0` for
/// [`Agent`](Self::Agent), `1` for [`Forbidden`](Self::Forbidden), `2`
/// for [`Error`](Self::Error) — and the rest is that variant's own
/// JSON: one [`Agent`](super::Agent) for the first, nothing for the
/// second, the error for the third.
///
/// # A finish with nothing is an answer
///
/// Nothing matched, or the account's grants reach no agents at all: a
/// scope that finishes with no response before it is that answer, not a
/// failure. An [`Error`](Self::Error) is a failure: the daemon could
/// not list, for whatever reason it knows. What was sent before the
/// failure precedes the error; nothing follows it, and the list is not
/// whole.
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
    /// One matching agent, as the daemon holds it. Tag `0`.
    Agent(Agent),
    /// The account the request is served for holds no grant allowing
    /// it; nothing was sent. Tag `1`.
    Forbidden,
    /// A failure. Tag `2`.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Agent`].
const AGENT: u8 = 0;

/// Tag for [`Frame::Forbidden`].
const FORBIDDEN: u8 = 1;

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
            Frame::Agent(item) => {
                out.extend_from_slice(&[AGENT]);
                serde_json::to_writer(out, item)
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
    /// Four ways to fail, and each names which half failed.
    type Error = FrameError;

    // Spelled out for the same reason as `encode` above.
    fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            AGENT => serde_json::from_slice(rest).map(Frame::Agent).map_err(FrameError::Agent),
            FORBIDDEN => Ok(Frame::Forbidden),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A list response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's three.
    UnknownTag(u8),
    /// The agent did not parse.
    Agent(serde_json::Error),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("agents list response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown agents list response frame tag {tag}"),
            FrameError::Agent(error) => write!(f, "agents list agent did not parse: {error}"),
            FrameError::Error(error) => write!(f, "agents list error did not parse: {error}"),
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Agent(error) | FrameError::Error(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
