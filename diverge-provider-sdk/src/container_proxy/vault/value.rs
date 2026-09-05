//! The answer to a read.

use std::convert::Infallible;

use super::ResponseError;
use crate::encode::{Encode, Writer};

/// The one message on `/vault/get/{channel}`, before the close.
///
/// ```text
/// [kind: u8][value… | message…]
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Value<'a> {
    /// Kind `0`. The key's value, verbatim. Empty is a value.
    Present(&'a [u8]),
    /// Kind `1`. No such key.
    Missing,
    /// Kind `2`. The read was refused or failed, and this says why —
    /// see [`Done::Error`](super::Done::Error) for why it is a
    /// string.
    Error(&'a str),
}

impl Encode for Value<'_> {
    /// [`Infallible`]: a kind byte and bytes copied.
    type Error = Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Infallible> {
        match self {
            Value::Present(value) => {
                out.extend_from_slice(&[0]);
                out.extend_from_slice(value);
            }
            Value::Missing => out.extend_from_slice(&[1]),
            Value::Error(message) => {
                out.extend_from_slice(&[2]);
                out.extend_from_slice(message.as_bytes());
            }
        }
        Ok(())
    }
}

impl<'a> Value<'a> {
    /// Decode the one message. The value or message borrows from
    /// `bytes`.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, ResponseError> {
        let (kind, rest) = bytes.split_first().ok_or(ResponseError::Empty)?;
        match *kind {
            0 => Ok(Value::Present(rest)),
            1 => Ok(Value::Missing),
            2 => std::str::from_utf8(rest)
                .map(Value::Error)
                .map_err(|_| ResponseError::MessageUtf8),
            other => Err(ResponseError::UnknownKind(other)),
        }
    }
}
