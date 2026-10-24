//! What a server's response frame carries for an image listing.

use std::fmt;

use super::Image;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::error::Error;
use crate::shared::listing::Change;

/// An image the caller built, as it comes to be listed or goes; the
/// word that the listing is whole; or the news that the provider
/// could not say.
///
/// A listing is a stream kept open. First every image the caller has
/// built, each as [`Added`](Self::Added); then exactly one
/// [`Listed`](Self::Listed); then, for the scope's life, an image the
/// caller builds as `Added` and one it deletes as
/// [`Removed`](Self::Removed). Nothing is told as changed: an image
/// never changes. The finish follows the caller's stop, the caller's
/// connection ending, or exactly one [`Error`](Self::Error), and
/// nothing else: a caller with no image is listed as `Listed` at
/// once, and watched. A payload leads with one byte saying which —
/// `0` for `Added`, `1` for `Removed`, `2` for `Listed`, `3` for
/// `Unsupported`, `4` for `Error` — and the rest is that variant's
/// own bytes; `Listed` and `Unsupported` carry nothing. A provider
/// with no image store sends `Unsupported` first and alone, and the
/// finish follows it.
///
/// # A finish with nothing is an answer
///
/// The caller stopped before anything was sent, or the request was
/// not served: a scope that finishes with no response before it is
/// that, not a failure. An [`Error`](Self::Error) is a failure: the
/// provider could not tell.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// An image the caller built, listed. Tag `0`.
    ///
    /// Once per name while the image is listed: at the opening for
    /// every image the caller has then, and after it for one the
    /// caller builds. Nothing promises an order.
    Added(Image),
    /// An image listed already, gone. Tag `1`.
    ///
    /// The image as it was listed.
    Removed(Image),
    /// Every image the caller had when the scope opened has been
    /// listed. Tag `2`.
    ///
    /// Carries nothing — the variant is bare. A caller that wants the
    /// listing once stops here; one that watches reads on.
    Listed,
    /// The provider has no image store — its `images_store` is `false`,
    /// see [`info`](crate::provider::endpoints::info) — and nothing was
    /// done. Tag `3`.
    ///
    /// Carries nothing. A caller asks another provider; nothing about
    /// the request is the cause.
    Unsupported,
    /// A failure. Tag `4`.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Added`].
const ADDED: u8 = 0;

/// Tag for [`Frame::Removed`].
const REMOVED: u8 = 1;

/// Tag for [`Frame::Listed`].
const LISTED: u8 = 2;

/// Tag for [`Frame::Unsupported`].
const UNSUPPORTED: u8 = 3;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 4;

/// Two formats, and the tag chooses between them: an image is
/// postcard, as a volume is in its listing, and the error is JSON,
/// since a [`serde_json::Value`] cannot come back out of a format
/// with no self-description.
impl Encode for Frame {
    /// One failure per half, and they are different libraries'.
    type Error = FrameEncodeError;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), FrameEncodeError> {
        match self {
            Frame::Added(image) => {
                out.extend_from_slice(&[ADDED]);
                postcard::to_io(image, &mut *out).map(|_| ()).map_err(FrameEncodeError::Image)
            }
            Frame::Removed(image) => {
                out.extend_from_slice(&[REMOVED]);
                postcard::to_io(image, &mut *out).map(|_| ()).map_err(FrameEncodeError::Image)
            }
            Frame::Listed => {
                out.extend_from_slice(&[LISTED]);
                Ok(())
            }
            Frame::Unsupported => {
                out.extend_from_slice(&[UNSUPPORTED]);
                Ok(())
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
            ADDED => postcard::from_bytes(rest).map(Frame::Added).map_err(FrameDecodeError::Image),
            REMOVED => postcard::from_bytes(rest).map(Frame::Removed).map_err(FrameDecodeError::Image),
            LISTED => Ok(Frame::Listed),
            UNSUPPORTED => Ok(Frame::Unsupported),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameDecodeError::Error),
            tag => Err(FrameDecodeError::UnknownTag(tag)),
        }
    }
}

impl Frame {
    /// What this frame tells a listing — a [`Change`], for
    /// [`Listing::push`](crate::shared::listing::Listing::push) — or
    /// the frame itself where it tells a listing nothing:
    /// unsupported, or a failure, the answers that end the scope. No
    /// frame is `Changed`: an image whose digest differs is `Removed`
    /// and then `Added`.
    pub fn change(self) -> Result<Change<Image>, Frame> {
        match self {
            Frame::Added(item) => Ok(Change::Added(item)),
            Frame::Removed(item) => Ok(Change::Removed(item)),
            Frame::Listed => Ok(Change::Listed),
            other => Err(other),
        }
    }
}

/// An image listing response that could not be written.
#[derive(Debug)]
pub enum FrameEncodeError {
    /// The image did not serialize.
    Image(postcard::Error),
    /// The error did not serialize.
    Error(serde_json::Error),
}

impl fmt::Display for FrameEncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameEncodeError::Image(error) => write!(f, "image listing image did not serialize: {error}"),
            FrameEncodeError::Error(error) => write!(f, "image listing error did not serialize: {error}"),
        }
    }
}

impl std::error::Error for FrameEncodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameEncodeError::Image(error) => Some(error),
            FrameEncodeError::Error(error) => Some(error),
        }
    }
}

/// An image listing response that could not be read.
#[derive(Debug)]
pub enum FrameDecodeError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's five.
    UnknownTag(u8),
    /// The image did not parse.
    Image(postcard::Error),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameDecodeError::Empty => f.write_str("image listing response frame is empty"),
            FrameDecodeError::UnknownTag(tag) => write!(f, "unknown image listing response frame tag {tag}"),
            FrameDecodeError::Image(error) => write!(f, "image listing image did not parse: {error}"),
            FrameDecodeError::Error(error) => write!(f, "image listing error did not parse: {error}"),
        }
    }
}

impl std::error::Error for FrameDecodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameDecodeError::Image(error) => Some(error),
            FrameDecodeError::Error(error) => Some(error),
            FrameDecodeError::Empty | FrameDecodeError::UnknownTag(_) => None,
        }
    }
}
