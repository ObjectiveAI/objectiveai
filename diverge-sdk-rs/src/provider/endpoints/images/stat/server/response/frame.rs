//! What a server's response frame carries for an image stat.

use std::fmt;

use super::Stat;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::error::Error;

/// The image examined, or the news that it could not be.
///
/// One of these on channel `0`, then the scope finishes. A payload
/// leads with one byte saying which — `0` for [`Stat`](Self::Stat),
/// `1` for [`Error`](Self::Error) — and the rest is that variant's
/// own bytes.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The image examined. Tag `0`.
    Stat(Stat),
    /// A failure. Tag `1`.
    ///
    /// An image the caller has none by that name is one, and so is a
    /// provider that could not look. See
    /// [`shared::error::Error`](crate::shared::error::Error) for why
    /// it says so little.
    Error(Error),
}

/// Tag for [`Frame::Stat`].
const STAT: u8 = 0;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 1;

/// Two variants, two formats, and the tag chooses between them: the
/// stat is postcard, as the rest of the store is, and the
/// error is JSON, since a [`serde_json::Value`] cannot come back out
/// of a format with no self-description.
impl Encode for Frame {
    /// One failure per half, and they are different libraries'.
    type Error = FrameEncodeError;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), FrameEncodeError> {
        match self {
            Frame::Stat(stat) => {
                out.extend_from_slice(&[STAT]);
                postcard::to_io(stat, &mut *out).map(|_| ()).map_err(FrameEncodeError::Stat)
            }
            Frame::Error(error) => {
                out.extend_from_slice(&[ERROR]);
                error.encode(out).map_err(FrameEncodeError::Error)
            }
        }
    }
}

impl Decode<'_> for Frame {
    /// Four ways to fail, and each names which half failed.
    type Error = FrameDecodeError;

    // Spelled out for the same reason as `encode` above.
    fn decode(bytes: &[u8]) -> Result<Self, FrameDecodeError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameDecodeError::Empty)?;
        match *tag {
            STAT => postcard::from_bytes(rest).map(Frame::Stat).map_err(FrameDecodeError::Stat),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameDecodeError::Error),
            tag => Err(FrameDecodeError::UnknownTag(tag)),
        }
    }
}

/// An image stat that could not be written.
#[derive(Debug)]
pub enum FrameEncodeError {
    /// The stat did not serialize.
    Stat(postcard::Error),
    /// The error did not serialize.
    Error(serde_json::Error),
}

impl fmt::Display for FrameEncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameEncodeError::Stat(error) => write!(f, "image stat did not serialize: {error}"),
            FrameEncodeError::Error(error) => write!(f, "image stat error did not serialize: {error}"),
        }
    }
}

impl std::error::Error for FrameEncodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameEncodeError::Stat(error) => Some(error),
            FrameEncodeError::Error(error) => Some(error),
        }
    }
}

/// An image stat that could not be read.
#[derive(Debug)]
pub enum FrameDecodeError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is neither of this frame's two.
    UnknownTag(u8),
    /// The stat did not parse.
    Stat(postcard::Error),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameDecodeError::Empty => f.write_str("image stat response frame is empty"),
            FrameDecodeError::UnknownTag(tag) => write!(f, "unknown image stat response frame tag {tag}"),
            FrameDecodeError::Stat(error) => write!(f, "image stat did not parse: {error}"),
            FrameDecodeError::Error(error) => write!(f, "image stat error did not parse: {error}"),
        }
    }
}

impl std::error::Error for FrameDecodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameDecodeError::Stat(error) => Some(error),
            FrameDecodeError::Error(error) => Some(error),
            FrameDecodeError::Empty | FrameDecodeError::UnknownTag(_) => None,
        }
    }
}
