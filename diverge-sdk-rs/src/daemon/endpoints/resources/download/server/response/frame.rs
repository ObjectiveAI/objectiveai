//! What a server's response frame carries for a download.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::daemon::download::{Chunk, ChunkError};
use crate::shared::error::Error;

/// A download's answer: one chunk, nothing is the one named, forbidden,
/// or a failure.
///
/// A download is a stream: zero or more chunks, each one piece of one
/// file with the file's path — see [`Chunk`] for the order and what a
/// path is relative to — then the finish; or exactly one
/// [`NotFound`](Self::NotFound), then the finish; or exactly one
/// [`Forbidden`](Self::Forbidden), then the finish; or chunks and then
/// exactly one error, then the finish. A payload leads with one byte
/// saying which — `0` for [`Chunk`](Self::Chunk), `1` for
/// [`NotFound`](Self::NotFound), `2` for
/// [`Forbidden`](Self::Forbidden), `3` for [`Error`](Self::Error) — and
/// the rest is that variant's own bytes: the chunk as [`Chunk`] lays it
/// out, nothing for the next two, the error's JSON for the last.
///
/// # A finish with nothing is an answer
///
/// A directory with no file in it: a scope that finishes with no
/// response before it is that answer, not a failure. A zero-byte file
/// is one chunk, never nothing. [`NotFound`](Self::NotFound) is an
/// ANSWER: no resource is the one named, or nothing is at the path, and
/// nothing was sent. An [`Error`](Self::Error) is a failure: the daemon
/// could not read, or could not go on reading, in its own words. Chunks
/// sent before the failure precede the error; none follow it, and what
/// was sent is not whole.
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
pub enum Frame<'a> {
    /// One piece of one file, with its path. Tag `0`.
    Chunk(Chunk<'a>),
    /// No resource is the one named, or nothing is at the path; nothing
    /// was sent. Tag `1`.
    NotFound,
    /// The account the request is served for holds no grant allowing
    /// it; nothing was sent. Tag `2`.
    Forbidden,
    /// A failure. Tag `3`.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Chunk`].
const CHUNK: u8 = 0;

/// Tag for [`Frame::NotFound`].
const NOT_FOUND: u8 = 1;

/// Tag for [`Frame::Forbidden`].
const FORBIDDEN: u8 = 2;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 3;

/// A tag, then the variant's own bytes, if it has any.
impl Encode for Frame<'_> {
    /// The ordinary JSON failure, from the chunk's path or the error.
    /// The bare answers cannot fail.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Chunk(chunk) => {
                out.extend_from_slice(&[CHUNK]);
                chunk.encode(out)
            }
            Frame::NotFound => {
                out.extend_from_slice(&[NOT_FOUND]);
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

impl<'a> Decode<'a> for Frame<'a> {
    /// Four ways to fail, and each names which half failed.
    type Error = FrameError;

    // Spelled out for the same reason as `encode` above.
    fn decode(bytes: &'a [u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            CHUNK => Chunk::decode(rest).map(Frame::Chunk).map_err(FrameError::Chunk),
            NOT_FOUND => Ok(Frame::NotFound),
            FORBIDDEN => Ok(Frame::Forbidden),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A download response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's four.
    UnknownTag(u8),
    /// The chunk did not parse.
    Chunk(ChunkError),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("resources download response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown resources download response frame tag {tag}"),
            FrameError::Chunk(error) => write!(f, "resources download chunk did not parse: {error}"),
            FrameError::Error(error) => write!(f, "resources download error did not parse: {error}"),
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Chunk(error) => Some(error),
            FrameError::Error(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
