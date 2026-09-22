//! What a server's response frame carries for a query.

use std::fmt;

use diverge_provider_sdk::decode::Decode;
use diverge_provider_sdk::encode::{Encode, Writer};
use diverge_provider_sdk::shared::error::Error;

use serde_json::Value;

/// A query's answer: one value the program yielded, or a failure.
///
/// A query is a stream: zero or more of these, each one value the jq
/// program yielded, in the order it yielded them over the log's items
/// oldest first, then the finish; or exactly one error, then the
/// finish. A payload leads with one byte saying which — `0` for
/// [`Value`](Self::Value), `1` for [`Error`](Self::Error) — and the
/// rest is that variant's own JSON. A value is whatever the program
/// made — an [`Item`](super::Item) when the program is `.`, a string,
/// a number, an object of the program's own — and this crate types
/// it as JSON; a reader that ran `.` reads each as an
/// [`Item`](super::Item).
///
/// # A finish with nothing is an answer
///
/// The program yielded nothing over the whole log, or the log holds
/// nothing at all: a scope that finishes with no response before it
/// is that answer, not a failure. An [`Error`](Self::Error) is a
/// failure: no agent of the client's has the name, the program would
/// not compile, or it failed while it ran, in the daemon's own words
/// or jq's. Values the program yielded before it failed precede the
/// error; none follow it.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// One value the program yielded. Tag `0`.
    Value(Value),
    /// A failure. Tag `1`.
    ///
    /// See [`shared::error::Error`](diverge_provider_sdk::shared::error::Error)
    /// for why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Value`].
const VALUE: u8 = 0;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 1;

/// A tag, then the variant's own JSON.
impl Encode for Frame {
    /// The ordinary JSON failure, from either half.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Value(value) => {
                out.extend_from_slice(&[VALUE]);
                serde_json::to_writer(out, value)
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
            VALUE => serde_json::from_slice(rest).map(Frame::Value).map_err(FrameError::Value),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A query response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is neither of this frame's two.
    UnknownTag(u8),
    /// The value did not parse.
    Value(serde_json::Error),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("agents logs query response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown agents logs query response frame tag {tag}"),
            FrameError::Value(error) => write!(f, "agents logs query value did not parse: {error}"),
            FrameError::Error(error) => write!(f, "agents logs query error did not parse: {error}"),
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Value(error) | FrameError::Error(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
