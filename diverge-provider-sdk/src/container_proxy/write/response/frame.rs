//! Whether the file landed.

use std::convert::Infallible;

use super::FrameError;
use crate::encode::{Encode, Writer};

/// The one message the container sends on `/write`, after the empty
/// chunk and before the close.
///
/// ```text
/// [kind: u8][message…]
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Frame<'a> {
    /// Kind `0`. The file is written, whole.
    Ok,
    /// Kind `1`. It is not, and this says why, for a reader rather
    /// than a program: a missing parent, a path that is a directory,
    /// a disk that is full. Nothing here is enumerated, because what
    /// a container refuses is its filesystem's business.
    Error(&'a str),
}

impl Encode for Frame<'_> {
    /// [`Infallible`]: a kind byte and bytes copied.
    type Error = Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Infallible> {
        match self {
            Frame::Ok => out.extend_from_slice(&[0]),
            Frame::Error(message) => {
                out.extend_from_slice(&[1]);
                out.extend_from_slice(message.as_bytes());
            }
        }
        Ok(())
    }
}

impl<'a> Frame<'a> {
    /// Decode the one message. The message borrows from `bytes`.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, FrameError> {
        let (kind, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *kind {
            0 => Ok(Frame::Ok),
            1 => std::str::from_utf8(rest)
                .map(Frame::Error)
                .map_err(|_| FrameError::MessageUtf8),
            other => Err(FrameError::UnknownKind(other)),
        }
    }
}
