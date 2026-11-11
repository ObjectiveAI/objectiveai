//! What a server's response frame carries for an image listing.

use std::fmt;

use super::Image;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::error::Error;
use crate::shared::listing::Change;

/// An image in the caller's Store, as it comes to be listed, changes,
/// or goes; the word that the listing is whole; or the news that the
/// provider could not say.
///
/// A listing is a stream kept open. First every image the caller
/// owns, each as [`Added`](Self::Added); then exactly one
/// [`Listed`](Self::Listed); then, for the scope's life, an image the
/// caller builds or pushes as `Added`, one whose description it edits
/// as [`Changed`](Self::Changed), and one it deletes as
/// [`Removed`](Self::Removed). The finish follows the caller's cancel,
/// the caller's connection ending, or exactly one
/// [`Error`](Self::Error), and nothing else: a caller with no image is
/// listed as `Listed` at once, and watched. A payload leads with one
/// byte saying which — `0` for `Added`, `1` for `Changed`, `2` for
/// `Removed`, `3` for `Listed`, `4` for `Unsupported`, `5` for
/// `Error`, `6` for `AlreadyOpen` — and the rest is that variant's own
/// bytes; `Listed`, `Unsupported` and `AlreadyOpen` carry nothing. A
/// provider with no Store sends `Unsupported` first and alone, and
/// the finish follows it; a connection with a listing open already is
/// sent [`AlreadyOpen`](Self::AlreadyOpen) the same way.
///
/// # A finish with nothing is an answer
///
/// The caller cancelled before anything was sent, or the request was
/// not served: a scope that finishes with no response before it is
/// that, not a failure. An [`Error`](Self::Error) is a failure: the
/// provider could not tell.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// An image in the caller's Store, listed. Tag `0`.
    ///
    /// Once per digest while the image is listed: at the opening for
    /// every image the caller owns then, and after it for one the
    /// caller builds or pushes. Nothing promises an order.
    Added(Image),
    /// An image listed already, whose description the caller edited.
    /// Tag `1`.
    ///
    /// The image as it is now. Its `digest`, `bytes` and `created` are
    /// what they were: none of them changes.
    Changed(Image),
    /// An image listed already, gone. Tag `2`.
    ///
    /// The image as it was last listed.
    Removed(Image),
    /// Every image the caller had when the scope opened has been
    /// listed. Tag `3`.
    ///
    /// Carries nothing — the variant is bare. A caller that wants the
    /// listing once stops here; one that watches reads on.
    Listed,
    /// The provider has no Store — its `images_store` is `false`,
    /// see [`info`](crate::provider::endpoints::info) — and nothing was
    /// done. Tag `4`.
    ///
    /// Carries nothing. A caller asks another provider; nothing about
    /// the request is the cause.
    Unsupported,
    /// A failure. Tag `5`.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little.
    Error(Error),
    /// Refused: this connection has an image listing open already. Tag
    /// `6`.
    ///
    /// Carries nothing. Sent first and alone, and the finish follows
    /// it: nothing was listed. One connection holds at most one
    /// listing at a time — see the
    /// [endpoints](crate::provider::endpoints#one-open-at-a-time) —
    /// and the caller reads the one it has.
    AlreadyOpen,
}

/// Tag for [`Frame::Added`].
const ADDED: u8 = 0;

/// Tag for [`Frame::Changed`].
const CHANGED: u8 = 1;

/// Tag for [`Frame::Removed`].
const REMOVED: u8 = 2;

/// Tag for [`Frame::Listed`].
const LISTED: u8 = 3;

/// Tag for [`Frame::Unsupported`].
const UNSUPPORTED: u8 = 4;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 5;

/// Tag for [`Frame::AlreadyOpen`].
const ALREADY_OPEN: u8 = 6;

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
            Frame::Changed(image) => {
                out.extend_from_slice(&[CHANGED]);
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
            Frame::AlreadyOpen => {
                out.extend_from_slice(&[ALREADY_OPEN]);
                Ok(())
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
            CHANGED => postcard::from_bytes(rest).map(Frame::Changed).map_err(FrameDecodeError::Image),
            REMOVED => postcard::from_bytes(rest).map(Frame::Removed).map_err(FrameDecodeError::Image),
            LISTED => Ok(Frame::Listed),
            UNSUPPORTED => Ok(Frame::Unsupported),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameDecodeError::Error),
            ALREADY_OPEN => Ok(Frame::AlreadyOpen),
            tag => Err(FrameDecodeError::UnknownTag(tag)),
        }
    }
}

impl Frame {
    /// What this frame tells a listing — a [`Change`], for
    /// [`Listing::push`](crate::shared::listing::Listing::push) — or
    /// the frame itself where it tells a listing nothing:
    /// unsupported, already open, or a failure, the answers that end
    /// the scope.
    pub fn change(self) -> Result<Change<Image>, Frame> {
        match self {
            Frame::Added(item) => Ok(Change::Added(item)),
            Frame::Changed(item) => Ok(Change::Changed(item)),
            Frame::Removed(item) => Ok(Change::Removed(item)),
            Frame::Listed => Ok(Change::Listed),
            other => Err(other),
        }
    }
}

/// Every change has a frame: what a server sends for what its
/// listing tells.
impl From<Change<Image>> for Frame {
    fn from(change: Change<Image>) -> Frame {
        match change {
            Change::Added(item) => Frame::Added(item),
            Change::Changed(item) => Frame::Changed(item),
            Change::Removed(item) => Frame::Removed(item),
            Change::Listed => Frame::Listed,
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
    /// A tag that is none of this frame's seven.
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
