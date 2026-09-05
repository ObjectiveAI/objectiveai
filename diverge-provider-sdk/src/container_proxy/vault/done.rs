//! The answer to an operation that either happened or did not.

use std::convert::Infallible;

use super::ResponseError;
use crate::encode::{Encode, Writer};

/// The one message on `/vault/set/{channel}`, `/vault/delete/
/// {channel}`, `/vault/lock/{channel}` and `/vault/unlock/{channel}`,
/// before the close.
///
/// ```text
/// [kind: u8][message…]
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Done<'a> {
    /// Kind `0`. It happened: the key is written or removed, the lock
    /// is held or released.
    Ok,
    /// Kind `1`. It did not, and this says why, for a reader rather
    /// than a program: nothing here is enumerated, because what a
    /// vault can refuse is the caller's policy and not this
    /// specification's.
    Error(&'a str),
}

impl Encode for Done<'_> {
    /// [`Infallible`]: a kind byte and bytes copied.
    type Error = Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Infallible> {
        match self {
            Done::Ok => out.extend_from_slice(&[0]),
            Done::Error(message) => {
                out.extend_from_slice(&[1]);
                out.extend_from_slice(message.as_bytes());
            }
        }
        Ok(())
    }
}

impl<'a> Done<'a> {
    /// Decode the one message. The message borrows from `bytes`.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, ResponseError> {
        let (kind, rest) = bytes.split_first().ok_or(ResponseError::Empty)?;
        match *kind {
            0 => Ok(Done::Ok),
            1 => std::str::from_utf8(rest)
                .map(Done::Error)
                .map_err(|_| ResponseError::MessageUtf8),
            other => Err(ResponseError::UnknownKind(other)),
        }
    }
}
