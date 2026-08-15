//! What a server's channel request frame carries for a connection.

use std::error::Error;
use std::fmt;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::shared::container::write_bytes;

/// Send the content for a write.
///
/// What a provider asks a connector for, and never unprompted. A
/// provider wants nothing from a connector on its own account — the
/// image was somebody else's problem and so is deciding who may attach.
/// This exists only because a write cannot carry its own content: only
/// a responder can finish a channel, so the bytes have to travel as
/// responses on a channel the provider opened.
///
/// # A struct, and still a tag byte
///
/// One thing to ask for is a struct; an enum of one variant would be a
/// discriminant with nothing to discriminate.
///
/// The byte stays anyway, for the same reason
/// [`write_path`](crate::shared::container::write_path::response::Frame)
/// spends one: a second thing to ask a connector for is additive if
/// there is a tag to add to, and a wire break if there is not. Whoever
/// adds one turns this into an enum with `Write` at tag `0` and changes
/// nothing on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Frame(
    /// Which write, by the channel the connector opened it on.
    pub write_bytes::request::Request,
);

/// The tag that says this is a request for a write's content.
const WRITE: u8 = 0;

impl Encode for Frame {
    /// [`Infallible`](std::convert::Infallible): a tag and four known
    /// bytes.
    type Error = std::convert::Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        out.extend_from_slice(&[WRITE]);
        self.0.encode(out)
    }
}

impl Decode<'_> for Frame {
    /// Three ways to fail, and none of them is a parse.
    type Error = FrameError;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        if *tag != WRITE {
            return Err(FrameError::UnknownTag(*tag));
        }
        write_bytes::request::Request::decode(rest)
            .map(Frame)
            .map_err(FrameError::Write)
    }
}

/// A connection channel request that could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag this version does not define.
    ///
    /// Which is what a provider asking for something else will send,
    /// once there is something else to ask for. Until then it is a
    /// peer that disagrees about the protocol.
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
