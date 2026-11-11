//! What a server's response frame carries for an image edit.

use std::fmt;

use crate::shared::error::Error;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// The description is kept, or the caller owns no such image, or the
/// provider has no Store, or it could not.
///
/// One of these on channel `0`, then the scope finishes. A payload
/// leads with one byte saying which — `0` for [`Edited`](Self::Edited),
/// `1` for [`NotFound`](Self::NotFound), `2` for
/// [`Unsupported`](Self::Unsupported), `3` for [`Error`](Self::Error)
/// — and for the first three there is nothing after it, because
/// saying so IS the whole message.
///
/// | the scope ends with | means |
/// |----------------------|-------|
/// | [`Edited`](Self::Edited), then a finish | the image has the description from now on, and the listing says `Changed` |
/// | [`NotFound`](Self::NotFound), then a finish | the caller owns no image by the digest, and nothing was changed |
/// | [`Unsupported`](Self::Unsupported), then a finish | the provider has no Store, and there was nothing to edit |
/// | an [`Error`](Self::Error), then a finish | it is not kept, for some other reason, and a caller should assume the description is as it was |
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The description is kept. Tag `0`.
    Edited,
    /// The caller owns no image by the digest, and nothing was
    /// changed. Tag `1`.
    NotFound,
    /// The provider has no Store — its `images_store` is `false`, see
    /// [`info`](crate::provider::endpoints::info) — and there was
    /// nothing to edit. Tag `2`.
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

/// Tag for [`Frame::Edited`].
const EDITED: u8 = 0;

/// Tag for [`Frame::NotFound`].
const NOT_FOUND: u8 = 1;

/// Tag for [`Frame::Unsupported`].
const UNSUPPORTED: u8 = 2;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 3;

/// A tag, and for a failure the JSON after it: an
/// [`Error`](Frame::Error) is a [`serde_json::Value`], which cannot
/// come back out of a format with no self-description, and the other
/// three are the tag alone.
impl Encode for Frame {
    /// The ordinary JSON failure, from the one variant that has one.
    /// A lone tag byte cannot fail.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Edited => {
                out.extend_from_slice(&[EDITED]);
                Ok(())
            }
            Frame::NotFound => {
                out.extend_from_slice(&[NOT_FOUND]);
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
            EDITED => Ok(Frame::Edited),
            NOT_FOUND => Ok(Frame::NotFound),
            UNSUPPORTED => Ok(Frame::Unsupported),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// An image edit result that could not be read.
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
            FrameError::Empty => f.write_str("image edit result frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown image edit result frame tag {tag}"),
            FrameError::Error(error) => write!(f, "image edit error did not parse: {error}"),
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
