//! What a server's response frame carries for a query.

use std::fmt;

use diverge_provider_sdk::decode::Decode;
use diverge_provider_sdk::encode::{Encode, Writer};
use diverge_provider_sdk::shared::error::Error;

use super::Item;

/// A query's answer: one item, or a failure.
///
/// A query is a stream: zero or more of these, each one item of the
/// log after the requested `log_id`, oldest first, then the finish; or
/// exactly one error, then the finish. A payload leads with one byte
/// saying which — `0` for [`Item`](Self::Item), `1` for
/// [`Error`](Self::Error) — and the rest is that variant's own JSON.
///
/// # A finish with nothing is an answer
///
/// Nothing lies after the requested id, and the log may hold nothing
/// at all: a scope that finishes with no response before it is that
/// answer, not a failure. An [`Error`](Self::Error) is a failure: no
/// agent of the client's has the name, or the log could not be read,
/// in the daemon's own words. No item precedes an error and none
/// follows it.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// One item of the log. Tag `0`.
    Item(Item),
    /// A failure. Tag `1`.
    ///
    /// See [`shared::error::Error`](diverge_provider_sdk::shared::error::Error)
    /// for why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Item`].
const ITEM: u8 = 0;

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
            Frame::Item(item) => {
                out.extend_from_slice(&[ITEM]);
                serde_json::to_writer(out, item)
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
            ITEM => serde_json::from_slice(rest).map(Frame::Item).map_err(FrameError::Item),
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
    /// The item did not parse.
    Item(serde_json::Error),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("agents logs query response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown agents logs query response frame tag {tag}"),
            FrameError::Item(error) => write!(f, "agents logs query item did not parse: {error}"),
            FrameError::Error(error) => write!(f, "agents logs query error did not parse: {error}"),
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Item(error) | FrameError::Error(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
