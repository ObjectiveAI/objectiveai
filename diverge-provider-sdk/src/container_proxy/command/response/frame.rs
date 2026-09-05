//! One item the command produced, or the news that it will not
//! produce another.

use super::FrameError;
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
pub enum Frame<'a> {
    /// One item, borrowed from the message it arrived in. Tag `0`.
    Item(&'a [u8]),
    /// The command did not finish. Tag `1`.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Item`].
const ITEM: u8 = 0;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 1;

impl Encode for Frame<'_> {
    /// The ordinary JSON failure, from the only variant that has one.
    /// An item is bytes copied.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Item(item) => {
                out.extend_from_slice(&[ITEM]);
                out.extend_from_slice(item);
                Ok(())
            }
            Frame::Error(error) => {
                out.extend_from_slice(&[ERROR]);
                error.encode(out)
            }
        }
    }
}

impl<'a> Frame<'a> {
    /// Decode one message. An item borrows from `bytes`.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            ITEM => Ok(Frame::Item(rest)),
            ERROR => Error::decode(rest)
                .map(super::Frame::Error)
                .map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}
