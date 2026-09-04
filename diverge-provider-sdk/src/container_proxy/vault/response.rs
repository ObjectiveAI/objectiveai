//! What the vault answers.

use std::error;
use std::fmt;

use crate::encode::{Encode, Writer};

/// The one response on a vault channel, before the finish.
///
/// ```text
/// [kind: u8][payload…]
/// ```
///
/// Which kinds a request may be answered with is stated on each
/// [`Request`](super::request::Request) variant. The server's
/// [`ChannelResponse`](super::server::Frame::ChannelResponse)
/// carries this as its payload; the opener decodes it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Response<'a> {
    /// Kind `0`. Done: the set, delete or unlock happened, or the
    /// lock is held.
    Ok,
    /// Kind `1`. The key's value, verbatim. Empty is a value.
    Value(&'a [u8]),
    /// Kind `2`. No such key.
    Missing,
    /// Kind `3`. The request was refused or failed, and this says
    /// why, for a reader rather than a program: nothing here is
    /// enumerated, because what a vault can refuse is the caller's
    /// policy and not this specification's.
    Error(&'a str),
}

impl Encode for Response<'_> {
    /// [`Infallible`](std::convert::Infallible): a kind byte and
    /// bytes copied.
    type Error = std::convert::Infallible;

    fn encode(
        &self,
        out: &mut Writer<'_>,
    ) -> Result<(), std::convert::Infallible> {
        match self {
            Response::Ok => out.extend_from_slice(&[0]),
            Response::Value(value) => {
                out.extend_from_slice(&[1]);
                out.extend_from_slice(value);
            }
            Response::Missing => out.extend_from_slice(&[2]),
            Response::Error(message) => {
                out.extend_from_slice(&[3]);
                out.extend_from_slice(message.as_bytes());
            }
        }
        Ok(())
    }
}

impl<'a> Response<'a> {
    /// Decode one response from a server frame's payload. The value
    /// or message borrows from `bytes`.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, ResponseError> {
        let (kind, rest) = bytes.split_first().ok_or(ResponseError::Empty)?;
        match *kind {
            0 => Ok(Response::Ok),
            1 => Ok(Response::Value(rest)),
            2 => Ok(Response::Missing),
            3 => std::str::from_utf8(rest)
                .map(Response::Error)
                .map_err(|_| ResponseError::MessageUtf8),
            other => Err(ResponseError::UnknownKind(other)),
        }
    }
}

/// A vault response that could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResponseError {
    /// No bytes at all, so not even a kind.
    Empty,
    /// A kind this path does not define.
    UnknownKind(u8),
    /// An error message that is not UTF-8.
    MessageUtf8,
}

impl fmt::Display for ResponseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResponseError::Empty => f.write_str("vault response is empty"),
            ResponseError::UnknownKind(kind) => {
                write!(f, "unknown vault response kind {kind}")
            }
            ResponseError::MessageUtf8 => {
                f.write_str("vault error message is not utf-8")
            }
        }
    }
}

impl error::Error for ResponseError {}
