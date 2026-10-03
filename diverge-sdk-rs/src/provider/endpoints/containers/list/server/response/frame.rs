//! What a server's response frame carries for a listing.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::error::Error;

use super::Container;

/// A listing's answer: one container, or a failure.
///
/// A listing is a stream: zero or more of these, one per container
/// the identity runs whose runner said yes, each sent the moment
/// that runner answered and in no order a lister may rely on, then
/// the finish once every runner has answered or gone; or exactly one
/// error, then the finish. A payload leads with one byte saying
/// which — `0` for [`Container`](Self::Container), `1` for
/// [`Error`](Self::Error) — and the rest is that variant's own JSON.
///
/// # A finish with nothing is an answer
///
/// The identity runs nothing, or nothing its runners let this lister
/// see: a scope that finishes with no response before it is that
/// answer, not a failure, and the two cases are not told apart. An
/// [`Error`](Self::Error) is a failure: the provider could not ask,
/// for whatever reason it knows. Containers sent before the failure
/// precede the error; none follow it, and the listing is not whole.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// One container. Tag `0`.
    Container(Container),
    /// A failure. Tag `1`.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error)
    /// for why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Container`].
const CONTAINER: u8 = 0;

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
            Frame::Container(container) => {
                out.extend_from_slice(&[CONTAINER]);
                serde_json::to_writer(out, container)
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
            CONTAINER => serde_json::from_slice(rest).map(Frame::Container).map_err(FrameError::Container),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A listing response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is neither of this frame's two.
    UnknownTag(u8),
    /// The container did not parse.
    Container(serde_json::Error),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("containers list response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown containers list response frame tag {tag}"),
            FrameError::Container(error) => write!(f, "containers list container did not parse: {error}"),
            FrameError::Error(error) => write!(f, "containers list error did not parse: {error}"),
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Container(error) | FrameError::Error(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
