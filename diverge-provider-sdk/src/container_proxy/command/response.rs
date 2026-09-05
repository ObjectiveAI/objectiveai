//! One item the command produced, or the news that it will not
//! produce another.

use std::error;
use std::fmt;

use crate::decode::Decode as _;
use crate::encode::{Encode, Writer};
use crate::shared::error::Error;

/// One message on `/command/{channel}`.
///
/// A message leads with one byte saying which — `0` for
/// [`Item`](Self::Item), `1` for [`Error`](Self::Error) — and the
/// rest is that variant's own bytes.
///
/// # An error is not an item
///
/// And the tag is what keeps them apart. Without it a command that
/// failed would have to say so IN an item, in whatever shape the CLI
/// used for that, and a container that did not know the shape could
/// not find out. [`Error`](Self::Error) is the caller saying the
/// command did not finish. What arrived before it is what the
/// command produced; nothing follows it, because the close comes
/// after.
///
/// # The item is opaque, and the tag does not change that
///
/// The tag says whether there is an item, not what is in one.
#[derive(Debug, Clone, PartialEq)]
pub enum Response<'a> {
    /// One item, borrowed from the message it arrived in. Tag `0`.
    Item(&'a [u8]),
    /// The command did not finish. Tag `1`.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little.
    Error(Error),
}

/// Tag for [`Response::Item`].
const ITEM: u8 = 0;

/// Tag for [`Response::Error`].
const ERROR: u8 = 1;

impl Encode for Response<'_> {
    /// The ordinary JSON failure, from the only variant that has one.
    /// An item is bytes copied.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Response::Item(item) => {
                out.extend_from_slice(&[ITEM]);
                out.extend_from_slice(item);
                Ok(())
            }
            Response::Error(error) => {
                out.extend_from_slice(&[ERROR]);
                error.encode(out)
            }
        }
    }
}

impl<'a> Response<'a> {
    /// Decode one message. An item borrows from `bytes`.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, ResponseError> {
        let (tag, rest) = bytes.split_first().ok_or(ResponseError::Empty)?;
        match *tag {
            ITEM => Ok(Response::Item(rest)),
            ERROR => Error::decode(rest)
                .map(super::Response::Error)
                .map_err(ResponseError::Error),
            tag => Err(ResponseError::UnknownTag(tag)),
        }
    }
}

/// A command response that could not be read.
#[derive(Debug)]
pub enum ResponseError {
    /// No bytes at all, so not even a tag.
    ///
    /// Distinct from an empty item, which is a tag byte followed by
    /// nothing and is a command that produced something with no bytes
    /// in it.
    Empty,
    /// A tag that is neither of this response's two.
    UnknownTag(u8),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for ResponseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResponseError::Empty => {
                f.write_str("command response is empty")
            }
            ResponseError::UnknownTag(tag) => {
                write!(f, "unknown command response tag {tag}")
            }
            ResponseError::Error(error) => {
                write!(f, "command error did not parse: {error}")
            }
        }
    }
}

impl error::Error for ResponseError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            ResponseError::Error(error) => Some(error),
            ResponseError::Empty | ResponseError::UnknownTag(_) => None,
        }
    }
}
