//! Ok, or why not.

use std::convert::Infallible;

use super::super::ResponseError;
use crate::encode::{Encode, Writer};

/// The one message that answers a mutation.
///
/// ```text
/// [kind: u8][message…]
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Frame<'a> {
    /// Kind `0`. It happened: the caller holds the result.
    Ok,
    /// Kind `1`. It did not, and this says why, for a reader rather
    /// than a program: what a caller can refuse — a mount it keeps
    /// unchangeable, a directory it will not empty, a path it will not
    /// serve — is its policy and not this specification's.
    Error(&'a str),
    /// Kind `2`. It did not, because the storage behind the mount
    /// keeps nothing written into it: a volume whose persist mode is
    /// `false`, served live. Nothing after the kind. The program sees
    /// a read-only filesystem, `EROFS`, and every immutable ask on the
    /// same mount goes through.
    Ephemeral,
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
            Frame::Ephemeral => out.extend_from_slice(&[2]),
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
            2 => Ok(Frame::Ephemeral),
            other => Err(ResponseError::UnknownKind(other)),
        }
    }
}
