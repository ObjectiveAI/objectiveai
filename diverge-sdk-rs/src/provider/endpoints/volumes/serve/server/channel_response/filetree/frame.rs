//! What a server's channel response frame carries on a serve's
//! filetree channel.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::error::Error;
use crate::shared::filetree;

/// One change on the served volume, or the news that the tree cannot
/// be watched.
///
/// A payload leads with one byte saying which — `0` for
/// [`Filetree`](Self::Filetree), `1` for [`Error`](Self::Error) — and
/// the rest is that variant's own bytes.
///
/// # How it ends
///
/// | the channel ends with | means |
/// |-----------------------|-------|
/// | frames, then a finish | the scope ended, and the watch with it |
/// | an [`Error`](Self::Error), then a finish | the tree could not be watched, or is no longer |
///
/// A [`Snapshot`](crate::shared::filetree::response::Frame::Snapshot)
/// comes first and may come again — see
/// [`shared::filetree`](crate::shared::filetree) for when.
///
/// # The tree is the serve's view
///
/// The snapshot is the volume as this serve sees it: for a persistent
/// volume the volume itself, for an ephemeral one the volume under
/// the serve's layer, for a read-only one the volume, which no ask
/// changes. The changes after it are the ones this serve's own asks
/// make — a write, a truncate, a setattr, a mkdir, a remove, a rename
/// answered `ok` — reported as the proxy's own tree reports them: a
/// node complete, a rename as a removal and an insertion. Nothing
/// else changes what a serve sees, so nothing else is reported, and
/// a read-only volume's stream is the snapshot alone. Every directory
/// in the snapshot has `changes` `true`.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// One change on the served volume. Tag `0`.
    ///
    /// A [`filetree`](crate::shared::filetree) stream over the
    /// volume's root — one snapshot, then one frame per change.
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

/// Two variants, two encodings, and neither converted into the
/// other's: a filetree frame is postcard's and is handed to postcard;
/// an error is JSON, because a [`serde_json::Value`] cannot come back
/// out of postcard at all.
impl Encode for Frame {
    /// One failure per half, and they are different libraries'.
    type Error = FrameEncodeError;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), FrameEncodeError> {
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

/// A serve filetree response that could not be written.
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
            FrameEncodeError::Filetree(error) => write!(f, "serve filetree frame did not serialize: {error}"),
            FrameEncodeError::Error(error) => write!(f, "serve filetree error did not serialize: {error}"),
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

impl Decode<'_> for Frame {
    /// Four ways to fail, and each names which half failed.
    type Error = FrameError;

    // Spelled out for the same reason as `encode` above.
    fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            FILETREE => filetree::response::Frame::decode(rest).map(Frame::Filetree).map_err(FrameError::Filetree),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A serve filetree response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is neither of this frame's two.
    UnknownTag(u8),
    /// The filetree frame did not decode.
    Filetree(postcard::Error),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("serve filetree response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown serve filetree response frame tag {tag}"),
            FrameError::Filetree(error) => write!(f, "serve filetree frame did not decode: {error}"),
            FrameError::Error(error) => write!(f, "serve filetree error did not parse: {error}"),
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Filetree(error) => Some(error),
            FrameError::Error(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
