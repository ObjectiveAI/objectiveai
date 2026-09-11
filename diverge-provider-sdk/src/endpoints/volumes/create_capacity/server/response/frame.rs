//! What a server's response frame carries for a volume create-capacity question.

use std::fmt;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::shared::error::Error;

/// A number of bytes, or the news that the provider could not say.
///
/// One of these on channel `0`, then the scope finishes. A payload
/// leads with one byte saying which — `0` for
/// [`Capacity`](Self::Capacity), `1` for [`Error`](Self::Error) — and
/// the rest is that variant's own bytes.
///
/// # It is a fact about an instant, not a reservation
///
/// The number is the largest size, in bytes, a new volume of this
/// caller could have as of this answer.
/// Nothing is set aside by answering. The room the number describes
/// may go to another caller, or to another volume of this one,
/// between this answer and the request that relies on it — and that
/// request is then answered on its own terms.
///
/// `0` is a legal number: no volume can be made right now. It is an answer, not an [`Error`](Self::Error), which
/// means the provider could not tell.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The number of bytes, as of now. Tag `0`.
    Capacity(u64),
    /// A failure. Tag `1`.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Capacity`].
const CAPACITY: u8 = 0;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 1;

/// Two variants, two formats, and the tag chooses between them.
///
/// The number is **postcard** — one varint — matching the rest of
/// [`volumes`](crate::endpoints::volumes). The error is JSON, and has
/// to be: a [`serde_json::Value`] deserializes through
/// `deserialize_any`, which a format with no self-description cannot
/// answer.
impl Encode for Frame {
    /// One failure per half, and they are different libraries'.
    type Error = FrameEncodeError;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(
        &self,
        out: &mut Writer<'_>,
    ) -> Result<(), FrameEncodeError> {
        match self {
            Frame::Capacity(bytes) => {
                out.extend_from_slice(&[CAPACITY]);
                postcard::to_io(bytes, &mut *out)
                    .map(|_| ())
                    .map_err(FrameEncodeError::Capacity)
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
            CAPACITY => postcard::from_bytes(rest)
                .map(Frame::Capacity)
                .map_err(FrameDecodeError::Capacity),
            ERROR => Error::decode(rest)
                .map(Frame::Error)
                .map_err(FrameDecodeError::Error),
            tag => Err(FrameDecodeError::UnknownTag(tag)),
        }
    }
}

/// A volume create-capacity answer that could not be written.
#[derive(Debug)]
pub enum FrameEncodeError {
    /// The number did not serialize.
    Capacity(postcard::Error),
    /// The error did not serialize.
    Error(serde_json::Error),
}

impl fmt::Display for FrameEncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameEncodeError::Capacity(error) => {
                write!(f, "volume create-capacity did not serialize: {error}")
            }
            FrameEncodeError::Error(error) => {
                write!(f, "volume create-capacity error did not serialize: {error}")
            }
        }
    }
}

impl std::error::Error for FrameEncodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameEncodeError::Capacity(error) => Some(error),
            FrameEncodeError::Error(error) => Some(error),
        }
    }
}

/// A volume create-capacity answer that could not be read.
#[derive(Debug)]
pub enum FrameDecodeError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is neither of this frame's two.
    UnknownTag(u8),
    /// The number did not parse.
    Capacity(postcard::Error),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameDecodeError::Empty => {
                f.write_str("volume create-capacity response frame is empty")
            }
            FrameDecodeError::UnknownTag(tag) => {
                write!(f, "unknown volume create-capacity response frame tag {tag}")
            }
            FrameDecodeError::Capacity(error) => {
                write!(f, "volume create-capacity did not parse: {error}")
            }
            FrameDecodeError::Error(error) => {
                write!(f, "volume create-capacity error did not parse: {error}")
            }
        }
    }
}

impl std::error::Error for FrameDecodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameDecodeError::Capacity(error) => Some(error),
            FrameDecodeError::Error(error) => Some(error),
            FrameDecodeError::Empty | FrameDecodeError::UnknownTag(_) => None,
        }
    }
}
