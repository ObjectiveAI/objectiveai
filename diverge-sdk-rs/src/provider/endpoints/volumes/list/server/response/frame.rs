//! What a server's response frame carries for a volume listing.

use std::fmt;

use super::Volume;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::error::Error;

/// Every volume a provider will let this caller mount, or the news
/// that it could not say.
///
/// One of these on channel `0`, then the scope finishes. A payload
/// leads with one byte saying which — `0` for
/// [`Volumes`](Self::Volumes), `1` for [`Error`](Self::Error) — and
/// the rest is that variant's own bytes.
///
/// A listing is not a stream: a provider knows what it offers before
/// it is asked, so there is nothing to discover incrementally and
/// nothing to hold a channel open for.
///
/// An empty list is an ANSWER and means the provider offers nothing.
/// It is not an [`Error`](Self::Error), which means the provider could
/// not tell — a caller that confuses them stops asking when it should
/// retry.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The volumes, in whatever order the provider chose. Tag `0`.
    ///
    /// Nothing promises an order and nothing should be read into one.
    Volumes(Vec<Volume>),
    /// A failure. Tag `1`.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Volumes`].
const VOLUMES: u8 = 0;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 1;

/// Two variants, two formats, and the tag chooses between them.
///
/// The volumes are **postcard**, matching
/// [`filetree`](crate::shared::filetree) rather than the JSON the rest
/// of the crate uses: this relays nothing, so no byte of it has to
/// survive a round trip unchanged, and nothing downstream reads it as
/// text.
///
/// The error is JSON, and has to be. A
/// [`serde_json::Value`] deserializes through `deserialize_any`, which
/// a format with no self-description cannot answer — so postcard can
/// carry the listing and cannot carry the failure, and each variant
/// gets the format it needs.
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
            Frame::Volumes(volumes) => {
                out.extend_from_slice(&[VOLUMES]);
                postcard::to_io(volumes, &mut *out)
                    .map(|_| ())
                    .map_err(FrameEncodeError::Volumes)
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
            VOLUMES => postcard::from_bytes(rest)
                .map(Frame::Volumes)
                .map_err(FrameDecodeError::Volumes),
            ERROR => Error::decode(rest)
                .map(Frame::Error)
                .map_err(FrameDecodeError::Error),
            tag => Err(FrameDecodeError::UnknownTag(tag)),
        }
    }
}

/// A volume listing that could not be written.
#[derive(Debug)]
pub enum FrameEncodeError {
    /// The listing did not serialize.
    Volumes(postcard::Error),
    /// The error did not serialize.
    Error(serde_json::Error),
}

impl fmt::Display for FrameEncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameEncodeError::Volumes(error) => {
                write!(f, "volume listing did not serialize: {error}")
            }
            FrameEncodeError::Error(error) => {
                write!(f, "volume listing error did not serialize: {error}")
            }
        }
    }
}

impl std::error::Error for FrameEncodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameEncodeError::Volumes(error) => Some(error),
            FrameEncodeError::Error(error) => Some(error),
        }
    }
}

/// A volume listing that could not be read.
#[derive(Debug)]
pub enum FrameDecodeError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is neither of this frame's two.
    UnknownTag(u8),
    /// The listing did not parse.
    Volumes(postcard::Error),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameDecodeError::Empty => {
                f.write_str("volume listing response frame is empty")
            }
            FrameDecodeError::UnknownTag(tag) => {
                write!(f, "unknown volume listing response frame tag {tag}")
            }
            FrameDecodeError::Volumes(error) => {
                write!(f, "volume listing did not parse: {error}")
            }
            FrameDecodeError::Error(error) => {
                write!(f, "volume listing error did not parse: {error}")
            }
        }
    }
}

impl std::error::Error for FrameDecodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameDecodeError::Volumes(error) => Some(error),
            FrameDecodeError::Error(error) => Some(error),
            FrameDecodeError::Empty | FrameDecodeError::UnknownTag(_) => None,
        }
    }
}
