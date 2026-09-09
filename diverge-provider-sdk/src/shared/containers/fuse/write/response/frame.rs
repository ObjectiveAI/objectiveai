//! The answer to a write.

use std::convert::Infallible;

use super::super::super::ResponseError;
use crate::encode::{Encode, Writer};

/// The one message that answers a write.
///
/// ```text
/// [kind: u8][message…]
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Frame<'a> {
    /// Kind `0`. It happened: the caller holds the bytes.
    Ok,
    /// Kind `1`. It did not, and this says why, for a reader rather
    /// than a program: what a caller can refuse — a read-only mount it
    /// was written to anyway, above all — is its policy and not this
    /// specification's.
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
    pub fn decode(bytes: &'a [u8]) -> Result<Self, ResponseError> {
        let (kind, rest) = bytes.split_first().ok_or(ResponseError::Empty)?;
        match *kind {
            0 => Ok(Frame::Ok),
            1 => std::str::from_utf8(rest)
                .map(Frame::Error)
                .map_err(|_| ResponseError::MessageUtf8),
            other => Err(ResponseError::UnknownKind(other)),
        }
    }
}
