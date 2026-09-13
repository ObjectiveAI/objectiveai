//! What a server's response frame carries for a read.

use std::fmt;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::shared::containers::read;
use crate::shared::error::Error;

/// A piece of the file, or the news that there will not be one.
///
/// A payload leads with one byte saying which — `0` for
/// [`Body`](Self::Body), `1` for [`Error`](Self::Error) — and the rest
/// is that variant's own bytes.
///
/// # How it ends
///
/// | the scope ends with | means |
/// |---------------------|-------|
/// | bodies, then a finish | the bytes are the file |
/// | an [`Error`](Self::Error), then a finish | the file was not read, or not all of it |
///
/// Only a finish ends it. A zero-byte file is one empty body, then
/// the finish, so that a finish with nothing before it keeps its
/// standing meaning: could not serve, with nothing to say.
///
/// # The failure is this scope's, the bytes are not
///
/// [`Body`](Self::Body) carries
/// [`read::response::Frame`](crate::shared::containers::read::response::Frame),
/// which means the same thing wherever the exchange happens. The
/// [`Error`](Self::Error) beside it is the exchange's own.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame<'a> {
    /// A piece of the file. Tag `0`.
    ///
    /// Each at most [`CHUNK_SIZE`](crate::CHUNK_SIZE), in order, and
    /// every one appends to the file.
    Body(read::response::Frame<'a>),
    /// A failure. Tag `1`.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Body`].
const BODY: u8 = 0;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 1;

impl Encode for Frame<'_> {
    /// The ordinary JSON failure, from the half that has one. The
    /// other cannot fail.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(
        &self,
        out: &mut Writer<'_>,
    ) -> Result<(), serde_json::Error> {
        match self {
            Frame::Body(inner) => {
                out.extend_from_slice(&[BODY]);
                // Its error is `Infallible`, and an empty match on one
                // is how you say so: there is no value to handle.
                inner.encode(out).map_err(|error| match error {})
            }
            Frame::Error(error) => {
                out.extend_from_slice(&[ERROR]);
                error.encode(out)
            }
        }
    }
}

impl<'a> Decode<'a> for Frame<'a> {
    /// Three ways to fail, and only one of them is a parse.
    type Error = FrameError;

    // Spelled out for the same reason as `encode` above.
    fn decode(bytes: &'a [u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            BODY => read::response::Frame::decode(rest)
                .map(Frame::Body)
                .map_err(|error| match error {}),
            ERROR => {
                Error::decode(rest).map(Frame::Error).map_err(FrameError::Error)
            }
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A read response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    ///
    /// Distinct from a zero-length body, which is a tag followed by
    /// nothing and is a file.
    Empty,
    /// A tag that is neither of this frame's two.
    UnknownTag(u8),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("read response frame is empty"),
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown read response frame tag {tag}")
            }
            FrameError::Error(error) => {
                write!(f, "read error did not parse: {error}")
            }
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
