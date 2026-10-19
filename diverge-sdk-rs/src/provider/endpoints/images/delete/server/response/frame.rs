//! What a server's response frame carries for an image deletion.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::error::Error;

/// The image is gone, or a running container runs it and it stays,
/// or it is not gone for some other reason.
///
/// One of these on channel `0`, then the scope finishes. A payload
/// leads with one byte saying which — `0` for
/// [`Deleted`](Self::Deleted), `1` for [`InUse`](Self::InUse), `2`
/// for [`Unsupported`](Self::Unsupported), `3` for
/// [`Error`](Self::Error) — and for the first three there is nothing
/// after it, because saying so IS the whole message.
///
/// | the scope ends with | means |
/// |----------------------|-------|
/// | [`Deleted`](Self::Deleted), then a finish | the image is gone and a listing will not show it |
/// | [`InUse`](Self::InUse), then a finish | a running container of the caller runs it, and nothing was changed |
/// | [`Unsupported`](Self::Unsupported), then a finish | the provider has no image store, and there was nothing to delete |
/// | an [`Error`](Self::Error), then a finish | it is not gone, for some other reason, and a caller should assume it is intact |
///
/// # In use is an answer, not an error
///
/// A provider does not delete an image a running container of the
/// caller runs, and it says so with [`InUse`](Self::InUse) rather than
/// with an [`Error`](Self::Error) a caller could not tell from any
/// other failure. The distinction is what a caller acts on: an image
/// in use is one to stop the container over and ask again, and a
/// failure is not.
///
/// # Gone means gone
///
/// The image no longer exists, and the room it took is the store's
/// again. A caller that wants the name back with new content deletes
/// and builds — two asks, because they are two things.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The image is gone. Tag `0`.
    Deleted,
    /// A running container of the caller runs the image, and it was
    /// not deleted. Tag `1`.
    ///
    /// Nothing was changed: the image and the listing are as they
    /// were.
    InUse,
    /// The provider has no image store — its `images_store` is `false`,
    /// see [`info`](crate::provider::endpoints::info) — and there was
    /// nothing to delete. Tag `2`.
    ///
    /// Carries nothing. A caller asks another provider; nothing about
    /// the request is the cause.
    Unsupported,
    /// A failure. Tag `3`.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Deleted`].
const DELETED: u8 = 0;

/// Tag for [`Frame::InUse`].
const IN_USE: u8 = 1;

/// Tag for [`Frame::Unsupported`].
const UNSUPPORTED: u8 = 2;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 3;

/// A tag, and for a failure the JSON after it: an
/// [`Error`](Frame::Error) is a [`serde_json::Value`], which cannot
/// come back out of a format with no self-description, and the other
/// two are the tag alone.
impl Encode for Frame {
    /// The ordinary JSON failure, from the one variant that has one.
    /// A lone tag byte cannot fail.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Deleted => {
                out.extend_from_slice(&[DELETED]);
                Ok(())
            }
            Frame::InUse => {
                out.extend_from_slice(&[IN_USE]);
                Ok(())
            }
            Frame::Unsupported => {
                out.extend_from_slice(&[UNSUPPORTED]);
                Ok(())
            }
            Frame::Error(error) => {
                out.extend_from_slice(&[ERROR]);
                error.encode(out)
            }
        }
    }
}

impl Decode<'_> for Frame {
    /// Three ways to fail, and only one of them is a parse.
    type Error = FrameError;

    // Spelled out for the same reason as `encode` above.
    fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            DELETED => Ok(Frame::Deleted),
            IN_USE => Ok(Frame::InUse),
            UNSUPPORTED => Ok(Frame::Unsupported),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// An image deletion result that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's four.
    UnknownTag(u8),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("image deletion result frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown image deletion result frame tag {tag}"),
            FrameError::Error(error) => write!(f, "image deletion error did not parse: {error}"),
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Error(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
