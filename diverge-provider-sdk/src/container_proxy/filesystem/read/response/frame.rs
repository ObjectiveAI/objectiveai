//! A piece of the file, or why there will not be one.

use std::convert::Infallible;

use super::FrameError;
use crate::encode::{Encode, Writer};
use crate::shared::containers::read;

/// One message on `/filesystem/read`.
///
/// ```text
/// [kind: u8][bytes… | message…]
/// ```
///
/// A body is the shared read's own piece —
/// [`read::response::Frame`](crate::shared::containers::read::response::Frame),
/// bytes and nothing else — behind a kind byte, so an error can sit
/// beside it. The byte per chunk is the price of a read that can say
/// why it stopped, and it is small next to the chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Frame<'a> {
    /// Kind `0`. A piece of the file, in order. Empty is a piece: an
    /// empty file is one of these.
    Body(read::response::Frame<'a>),
    /// Kind `1`. The file was not read, or not all of it, and this
    /// says why, for a reader rather than a program: no such file, a
    /// directory, a read that failed partway. The last message before
    /// the close. Nothing here is enumerated, because what a
    /// filesystem refuses is its own business.
    Error(&'a str),
}

impl Encode for Frame<'_> {
    /// [`Infallible`]: a kind byte and bytes copied.
    type Error = Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Infallible> {
        match self {
            Frame::Body(body) => {
                out.extend_from_slice(&[0]);
                body.encode(out)
            }
            Frame::Error(message) => {
                out.extend_from_slice(&[1]);
                out.extend_from_slice(message.as_bytes());
                Ok(())
            }
        }
    }
}

impl<'a> Frame<'a> {
    /// Decode one message. The body or the message borrows from
    /// `bytes`.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, FrameError> {
        let (kind, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *kind {
            0 => Ok(Frame::Body(read::response::Frame(rest))),
            1 => std::str::from_utf8(rest)
                .map(Frame::Error)
                .map_err(|_| FrameError::MessageUtf8),
            other => Err(FrameError::UnknownKind(other)),
        }
    }
}
