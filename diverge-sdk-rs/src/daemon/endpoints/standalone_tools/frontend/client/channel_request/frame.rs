//! The channels a client opens on a frontend request.
//!
//! The rules:
//!
//! 1. `Body` carries a piece of the request's body — after a `101`, a
//!    piece of what the page sends — and is answered by the channel's
//!    finish, and nothing before it, once the daemon has taken it. A
//!    client sends its next `Body` once the last is finished, which is
//!    what keeps them in order.
//! 2. `End` ends the body, and is answered by the channel's finish.
//! 3. `Cancel` drops the request; the daemon answers nothing on the
//!    channel and finishes the scope.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// One channel on a frontend request: one byte of tag, then the bytes
/// as they are for `Body`, nothing for the rest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Frame<'a> {
    /// Tag `0`: a piece of the body.
    Body(&'a [u8]),
    /// Tag `1`: the body is done.
    End,
    /// Tag `2`: drop the request.
    Cancel,
}

const BODY: u8 = 0;
const END: u8 = 1;
const CANCEL: u8 = 2;

impl Encode for Frame<'_> {
    /// Nothing can go wrong.
    type Error = std::convert::Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        match self {
            Frame::Body(bytes) => {
                out.extend_from_slice(&[BODY]);
                out.extend_from_slice(bytes);
            }
            Frame::End => out.extend_from_slice(&[END]),
            Frame::Cancel => out.extend_from_slice(&[CANCEL]),
        }
        Ok(())
    }
}

impl<'a> Decode<'a> for Frame<'a> {
    /// See [`FrameError`].
    type Error = FrameError;

    fn decode(bytes: &'a [u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            BODY => Ok(Frame::Body(rest)),
            END => Ok(Frame::End),
            CANCEL => Ok(Frame::Cancel),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A channel request that did not decode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameError {
    /// No tag at all.
    Empty,
    /// A tag no variant is.
    UnknownTag(u8),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("standalone tools frontend channel request frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown standalone tools frontend channel request tag {tag}"),
        }
    }
}

impl std::error::Error for FrameError {}
