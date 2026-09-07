//! One change on the watched tree, or a failure to keep watching.

use std::fmt;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::shared::error::Error;
use crate::shared::filetree;

/// One change on the watched tree, or the news that there will not be
/// another.
///
/// A payload leads with one byte saying which — `0` for
/// [`Filetree`](Self::Filetree), `1` for [`Error`](Self::Error) — and
/// the rest is that variant's own bytes.
///
/// # The failure is this endpoint's, the tree is not
///
/// [`Filetree`](Self::Filetree) carries
/// [`filetree::response::Frame`](crate::shared::filetree::response::Frame),
/// which is what a change to a tree looks like anywhere — a
/// [`container`](crate::endpoints::containers) reports one over its
/// own filesystem using the same type. Defining it
/// again here would be two definitions of one thing waiting to
/// disagree, and
/// [`Root::update`](crate::shared::filetree::response::Root::update)
/// would be where they did.
///
/// The [`Error`](Self::Error) beside it is not shared, because what
/// can go wrong belongs to the exchange: a volume that went away under
/// a watch is this endpoint's problem, and a container that stopped is
/// the laboratory's.
///
/// # A watch is the one stream that does not expect to end
///
/// The others answer a question and finish. This one runs for as long
/// as the caller holds the scope, so an [`Error`](Self::Error) here is
/// the interesting case rather than the edge one — it is how a caller
/// learns that a tree it has been folding is no longer being kept up
/// to date, which it could not otherwise tell from quiet.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// One change on the tree. Tag `0`.
    ///
    /// The snapshot that establishes it, or one node inserted,
    /// modified, moved or removed. See
    /// [`filetree::response::Frame`](crate::shared::filetree::response::Frame)
    /// for the variants and for what makes the stream replay-safe.
    Filetree(filetree::response::Frame),
    /// A failure. Tag `1`.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Filetree`].
const FILETREE: u8 = 0;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 1;

/// Two variants, two formats, and the tag chooses between them. A
/// filetree frame is postcard's and is handed to postcard; an
/// [`Error`](Frame::Error) is a [`serde_json::Value`], which
/// deserializes through `deserialize_any` and so cannot come back out
/// of a format with no self-description.
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
            Frame::Filetree(frame) => {
                out.extend_from_slice(&[FILETREE]);
                frame.encode(out).map_err(FrameEncodeError::Filetree)
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
            FILETREE => filetree::response::Frame::decode(rest)
                .map(Frame::Filetree)
                .map_err(FrameDecodeError::Filetree),
            ERROR => Error::decode(rest)
                .map(Frame::Error)
                .map_err(FrameDecodeError::Error),
            tag => Err(FrameDecodeError::UnknownTag(tag)),
        }
    }
}

/// A watch response that could not be written.
#[derive(Debug)]
pub enum FrameEncodeError {
    /// The filetree frame did not serialize.
    Filetree(postcard::Error),
    /// The error did not serialize.
    Error(serde_json::Error),
}

impl fmt::Display for FrameEncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameEncodeError::Filetree(error) => {
                write!(f, "filetree frame did not serialize: {error}")
            }
            FrameEncodeError::Error(error) => {
                write!(f, "watch error did not serialize: {error}")
            }
        }
    }
}

impl std::error::Error for FrameEncodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameEncodeError::Filetree(error) => Some(error),
            FrameEncodeError::Error(error) => Some(error),
        }
    }
}

/// A watch response that could not be read.
#[derive(Debug)]
pub enum FrameDecodeError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is neither of this frame's two.
    UnknownTag(u8),
    /// The filetree frame did not decode.
    Filetree(postcard::Error),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameDecodeError::Empty => {
                f.write_str("watch response frame is empty")
            }
            FrameDecodeError::UnknownTag(tag) => {
                write!(f, "unknown watch response frame tag {tag}")
            }
            FrameDecodeError::Filetree(error) => {
                write!(f, "filetree frame did not decode: {error}")
            }
            FrameDecodeError::Error(error) => {
                write!(f, "watch error did not parse: {error}")
            }
        }
    }
}

impl std::error::Error for FrameDecodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameDecodeError::Filetree(error) => Some(error),
            FrameDecodeError::Error(error) => Some(error),
            FrameDecodeError::Empty | FrameDecodeError::UnknownTag(_) => None,
        }
    }
}
