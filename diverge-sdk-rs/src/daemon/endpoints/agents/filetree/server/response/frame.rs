//! What a server's response frame carries for a filetree.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::error::Error;
use crate::shared::filetree;

/// A filetree's answer: one change on the container's tree, no agent is
/// the one named, forbidden, or a failure.
///
/// A filetree is a stream: a snapshot of the whole container, then one
/// frame per change, for as long as the scope lives — until the client
/// cancels, or the agent is deleted — and then the finish; or exactly
/// one [`NotFound`](Self::NotFound), then the finish; or exactly one
/// [`Forbidden`](Self::Forbidden), then the finish; or frames and then
/// exactly one error, then the finish. A payload leads with one byte
/// saying which — `0` for [`Filetree`](Self::Filetree), `1` for
/// [`NotFound`](Self::NotFound), `2` for
/// [`Forbidden`](Self::Forbidden), `3` for [`Error`](Self::Error) — and
/// the rest is that variant's own bytes: a
/// [`filetree`](crate::shared::filetree) frame in postcard for the
/// first, as the provider protocol carries one, nothing for the next
/// two, the error's JSON for the last.
///
/// # The whole container
///
/// The tree is the container's root with every mount in it: the
/// provider's own tree of the container, which leaves the FUSE mounts
/// out, with each FUSE mount's subtree spliced in at its mount point by
/// the daemon, which serves every one of them from the filetree
/// channel of the `volumes::serve` it bridges the mount to. A change under a FUSE mount is reported as any other
/// change is, from the same source, and every directory under one has
/// `changes` `true`. Every path is from the container's root.
///
/// # A failure after frames
///
/// [`NotFound`](Self::NotFound) is an ANSWER: no agent is the one
/// named, and nothing was sent. An [`Error`](Self::Error) is a failure:
/// the daemon could not watch the container, or could no longer, in its
/// own words. Frames sent before the failure precede the error; none
/// follow it.
///
/// # Forbidden
///
/// [`Forbidden`](Self::Forbidden) is an answer every endpoint has: the
/// account the request is served for — the connection's, or the
/// `account` of the container it came from — holds no grant allowing
/// what the request asks over what it names, and nothing changed. Where
/// the answer is a stream it comes as an error does: exactly one, with
/// nothing before it, then the finish. Nothing is retried. See
/// [`grant`](crate::daemon::grant).
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// One change on the container's tree: the snapshot first, then one
    /// per change. Tag `0`.
    Filetree(filetree::response::Frame),
    /// No agent is the one named; nothing was sent. Tag `1`.
    NotFound,
    /// The account the request is served for holds no grant allowing
    /// it; nothing was sent. Tag `2`.
    Forbidden,
    /// A failure. Tag `3`.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Filetree`].
const FILETREE: u8 = 0;

/// Tag for [`Frame::NotFound`].
const NOT_FOUND: u8 = 1;

/// Tag for [`Frame::Forbidden`].
const FORBIDDEN: u8 = 2;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 3;

/// Two formats, and the tag chooses between them: a filetree frame is
/// postcard's, as it is everywhere a filetree travels; the error is
/// JSON, as every error of the daemon's is; the bare answers carry
/// nothing.
impl Encode for Frame {
    /// One failure per half, and they are different libraries'.
    type Error = FrameEncodeError;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), FrameEncodeError> {
        match self {
            Frame::Filetree(frame) => {
                out.extend_from_slice(&[FILETREE]);
                frame.encode(out).map_err(FrameEncodeError::Tree)
            }
            Frame::NotFound => {
                out.extend_from_slice(&[NOT_FOUND]);
                Ok(())
            }
            Frame::Forbidden => {
                out.extend_from_slice(&[FORBIDDEN]);
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
    type Error = FrameError;

    // Spelled out for the same reason as `encode` above.
    fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            FILETREE => filetree::response::Frame::decode(rest).map(Frame::Filetree).map_err(FrameError::Tree),
            NOT_FOUND => Ok(Frame::NotFound),
            FORBIDDEN => Ok(Frame::Forbidden),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A agents filetree response that could not be written.
#[derive(Debug)]
pub enum FrameEncodeError {
    /// The tree did not serialize.
    Tree(postcard::Error),
    /// The error did not serialize.
    Error(serde_json::Error),
}

impl fmt::Display for FrameEncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameEncodeError::Tree(error) => write!(f, "agents filetree tree did not serialize: {error}"),
            FrameEncodeError::Error(error) => write!(f, "agents filetree error did not serialize: {error}"),
        }
    }
}

impl std::error::Error for FrameEncodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameEncodeError::Tree(error) => Some(error),
            FrameEncodeError::Error(error) => Some(error),
        }
    }
}

/// A agents filetree response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's four.
    UnknownTag(u8),
    /// The filetree frame did not decode.
    Tree(postcard::Error),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("agents filetree response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown agents filetree response frame tag {tag}"),
            FrameError::Tree(error) => write!(f, "agents filetree frame did not decode: {error}"),
            FrameError::Error(error) => write!(f, "agents filetree error did not parse: {error}"),
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Tree(error) => Some(error),
            FrameError::Error(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
