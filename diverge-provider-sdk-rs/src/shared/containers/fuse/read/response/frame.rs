//! The answer to a read.

use std::convert::Infallible;

use super::super::super::ResponseError;
use crate::encode::{Encode, Writer};

/// The one message that answers a read.
///
/// ```text
/// [kind: u8][bytes… | message…]
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Frame<'a> {
    /// Kind `0`. The file's bytes, verbatim. Empty is a file.
    Present(&'a [u8]),
    /// Kind `1`. The caller holds nothing under the id yet: the file
    /// reads as empty, and the first write makes it.
    Missing,
    /// Kind `2`. The read was refused or failed, and this says why,
    /// for a reader rather than a program: what a caller can refuse
    /// is its policy and not this specification's.
    Error(&'a str),
}

impl Encode for Frame<'_> {
    /// [`Infallible`]: a kind byte and bytes copied.
    type Error = Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Infallible> {
        match self {
            Frame::Present(bytes) => {
                out.extend_from_slice(&[0]);
                out.extend_from_slice(bytes);
            }
            Frame::Missing => out.extend_from_slice(&[1]),
            Frame::Error(message) => {
                out.extend_from_slice(&[2]);
                out.extend_from_slice(message.as_bytes());
            }
        }
        Ok(())
    }
}

impl<'a> Frame<'a> {
    /// Decode the one message. The bytes or message borrow from
    /// `bytes`.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, ResponseError> {
        let (kind, rest) = bytes.split_first().ok_or(ResponseError::Empty)?;
        match *kind {
            0 => Ok(Frame::Present(rest)),
            1 => Ok(Frame::Missing),
            2 => std::str::from_utf8(rest)
                .map(Frame::Error)
                .map_err(|_| ResponseError::MessageUtf8),
            other => Err(ResponseError::UnknownKind(other)),
        }
    }
}
