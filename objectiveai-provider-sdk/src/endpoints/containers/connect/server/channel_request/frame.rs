//! What a server's channel request frame carries for a connection.

use std::error::Error;
use std::fmt;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::shared::container::write_bytes;

/// What a provider asks a connector for.
///
/// One thing, and never unprompted. A provider wants nothing from a
/// connector on its own account — the image was somebody else's
/// problem and so is deciding who may attach. This exists only because
/// a write cannot carry its own content: only a responder can finish a
/// channel, so the bytes have to travel as responses on a channel the
/// provider opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Frame {
    /// Send the content for a write. Tag `0`.
    Write(write_bytes::request::Request),
}

/// Tag for [`Frame::Write`].
const WRITE: u8 = 0;

impl Encode for Frame {
    /// [`Infallible`](std::convert::Infallible): a tag and four known
    /// bytes.
    type Error = std::convert::Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        match self {
            Frame::Write(request) => {
                out.extend_from_slice(&[WRITE]);
                request.encode(out)
            }
        }
    }
}

impl Decode<'_> for Frame {
    /// Three ways to fail, and none of them is a parse.
    type Error = FrameError;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            WRITE => write_bytes::request::Request::decode(rest)
                .map(Frame::Write)
                .map_err(FrameError::Write),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A connection channel request that could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is not [`Frame::Write`].
    UnknownTag(u8),
    /// The content request did not decode.
    Write(write_bytes::request::RequestError),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("connection channel request frame is empty")
            }
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown connection channel request tag {tag}")
            }
            FrameError::Write(error) => {
                write!(f, "write content request did not decode: {error}")
            }
        }
    }
}

impl Error for FrameError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            FrameError::Write(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
