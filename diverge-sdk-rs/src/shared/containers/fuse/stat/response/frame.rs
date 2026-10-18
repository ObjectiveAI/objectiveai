//! The answer to a stat.

use std::convert::Infallible;

use super::super::super::ResponseError;
use super::super::Stat;
use crate::wire::encode::{Encode, Writer};

/// The one message that answers a stat.
///
/// ```text
/// [kind: u8][stat… | message…]
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Frame<'a> {
    /// Kind `0`. What the entry is, how long, whose, and when: a
    /// [`Stat`].
    Present(Stat),
    /// Kind `1`. There is nothing at the path. On a file mount, the
    /// caller holds nothing under the id yet, and the file reads as
    /// empty — the same absence [`read`](super::super::super::read)
    /// reports.
    Missing,
    /// Kind `2`. The stat was refused or failed, and this says why,
    /// for a reader rather than a program.
    Error(&'a str),
}

impl Encode for Frame<'_> {
    /// [`Infallible`]: a kind byte and fixed bytes, or a message
    /// copied.
    type Error = Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Infallible> {
        match self {
            Frame::Present(stat) => {
                out.extend_from_slice(&[0]);
                stat.encode(out);
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
    /// Decode the one message. The message borrows from `bytes`.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, ResponseError> {
        let (kind, rest) = bytes.split_first().ok_or(ResponseError::Empty)?;
        match *kind {
            0 => Stat::decode(rest).map(|(stat, _)| Frame::Present(stat)),
            1 => Ok(Frame::Missing),
            2 => std::str::from_utf8(rest)
                .map(Frame::Error)
                .map_err(|_| ResponseError::MessageUtf8),
            other => Err(ResponseError::UnknownKind(other)),
        }
    }
}
