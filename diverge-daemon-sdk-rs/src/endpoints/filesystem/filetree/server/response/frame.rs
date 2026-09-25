//! What a server's response frame carries for a filesystem filetree.

use std::fmt;

use diverge_provider_sdk::decode::Decode;
use diverge_provider_sdk::encode::{Encode, Writer};
use diverge_provider_sdk::shared::error::Error;
use diverge_provider_sdk::shared::filetree;

/// One frame of the tree, or the news that there will not be another.
///
/// A payload leads with one byte saying which — `0` for
/// [`Filetree`](Self::Filetree), `1` for [`Error`](Self::Error) — and
/// the rest is that variant's own bytes.
///
/// # How it ends
///
/// | the scope ends with | means |
/// |---------------------|-------|
/// | frames, then a finish | the client cancelled, or closed |
/// | an [`Error`](Self::Error), then a finish | the watch could not be made, or died |
///
/// The first frame is a
/// [`Snapshot`](diverge_provider_sdk::shared::filetree::response::Frame::Snapshot)
/// of the directory, and every frame after it one change, or a fresh
/// snapshot when the daemon lost track. A finish with nothing before
/// it keeps its standing meaning: could not serve, with nothing to
/// say.
///
/// # The failure is this scope's, the frames are not
///
/// [`Filetree`](Self::Filetree) carries
/// [`filetree::response::Frame`](diverge_provider_sdk::shared::filetree::response::Frame),
/// which means the same thing wherever the exchange happens, in its
/// own postcard encoding. The [`Error`](Self::Error) beside it is the
/// exchange's own, as JSON: a path that is not absolute, nothing at
/// it, a file there, a watch the host refused.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// One frame of the tree. Tag `0`.
    Filetree(filetree::response::Frame),
    /// A failure. Tag `1`.
    ///
    /// See [`shared::error::Error`](diverge_provider_sdk::shared::error::Error)
    /// for why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Filetree`].
const FILETREE: u8 = 0;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 1;

/// Two variants, two formats, and the tag chooses between them: a
/// filetree frame is postcard, as it is everywhere, and the error is
/// JSON, as it has to be.
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
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameDecodeError::Error),
            tag => Err(FrameDecodeError::UnknownTag(tag)),
        }
    }
}

/// A filesystem filetree response that could not be written.
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
                write!(f, "filesystem filetree frame did not serialize: {error}")
            }
            FrameEncodeError::Error(error) => {
                write!(f, "filesystem filetree error did not serialize: {error}")
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

/// A filesystem filetree response that could not be read.
#[derive(Debug)]
pub enum FrameDecodeError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is neither of this frame's two.
    UnknownTag(u8),
    /// The filetree frame did not parse.
    Filetree(postcard::Error),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameDecodeError::Empty => f.write_str("filesystem filetree response frame is empty"),
            FrameDecodeError::UnknownTag(tag) => {
                write!(f, "unknown filesystem filetree response frame tag {tag}")
            }
            FrameDecodeError::Filetree(error) => {
                write!(f, "filesystem filetree frame did not parse: {error}")
            }
            FrameDecodeError::Error(error) => {
                write!(f, "filesystem filetree error did not parse: {error}")
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
