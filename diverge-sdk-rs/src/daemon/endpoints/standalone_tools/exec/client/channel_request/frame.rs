//! The cancel: the command killed.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// The one channel a client opens on an exec: cancel, which kills the
/// command; the scope's finish answers it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Frame {
    /// Tag `0`: kill the command.
    Cancel,
}

const CANCEL: u8 = 0;

impl Encode for Frame {
    /// Nothing can go wrong.
    type Error = std::convert::Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        match self {
            Frame::Cancel => out.extend_from_slice(&[CANCEL]),
        }
        Ok(())
    }
}

impl Decode<'_> for Frame {
    /// See [`FrameError`].
    type Error = FrameError;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        let (tag, _) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            CANCEL => Ok(Frame::Cancel),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A cancel that did not decode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameError {
    /// No tag at all.
    Empty,
    /// A tag no variant is.
    UnknownTag(u8),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("standalone tools exec channel request frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown standalone tools exec channel request tag {tag}"),
        }
    }
}

impl std::error::Error for FrameError {}
