//! What a server's response frame carries for a filetree.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::error::Error;
use crate::shared::filetree::response::Node;

/// A filetree's answer: the tree, no resource is the one named, a file
/// at the path, forbidden, or a failure.
///
/// A filetree is one question and one reply, so there is exactly one of
/// these per scope, before the finish that ends it. A payload leads
/// with one byte saying which — `0` for [`Tree`](Self::Tree), `1` for
/// [`NotFound`](Self::NotFound), `2` for
/// [`NotDirectory`](Self::NotDirectory), `3` for
/// [`Forbidden`](Self::Forbidden), `4` for [`Error`](Self::Error) — and
/// the rest is that variant's own bytes: the tree in postcard for the
/// first, as the provider protocol's `volumes::filetree` carries one,
/// nothing for the bare answers, the error's JSON for the last.
///
/// # A snapshot, not a watch
///
/// The tree is the entries of the directory the path names, each with
/// everything beneath it, as the [`Node`]s of a snapshot, once: nothing
/// follows, no change is ever reported, and every directory in it has
/// `changes` `false`. A caller that wants to know what changed asks
/// again, and compares. A resource never changes, so the tree is the
/// resource's for as long as the resource is held.
///
/// # Answers, and one failure
///
/// [`NotFound`](Self::NotFound), [`NotDirectory`](Self::NotDirectory)
/// are ANSWERS: the daemon looked, and no resource is the one named or
/// nothing is at the path, or what is there is a file, and in each case
/// nothing was walked and nothing is retried. An [`Error`](Self::Error)
/// is the absence of an answer: the daemon could not walk, for whatever
/// reason it knows.
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
    /// The tree: the entries beneath the path, each with everything
    /// beneath it. Tag `0`.
    Tree(Vec<Node>),
    /// No resource is the one named, or nothing is at the path; nothing
    /// was walked. Tag `1`.
    NotFound,
    /// What is at the path is a file, which has no tree; nothing was
    /// walked. Tag `2`.
    NotDirectory,
    /// The account the request is served for holds no grant allowing
    /// it; nothing was walked. Tag `3`.
    Forbidden,
    /// A failure. Tag `4`.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Tree`].
const TREE: u8 = 0;

/// Tag for [`Frame::NotFound`].
const NOT_FOUND: u8 = 1;

/// Tag for [`Frame::NotDirectory`].
const NOT_DIRECTORY: u8 = 2;

/// Tag for [`Frame::Forbidden`].
const FORBIDDEN: u8 = 3;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 4;

/// Two formats, and the tag chooses between them: the tree is
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
            Frame::Tree(tree) => {
                out.extend_from_slice(&[TREE]);
                postcard::to_io(tree, &mut *out).map_err(FrameEncodeError::Tree)?;
                Ok(())
            }
            Frame::NotFound => {
                out.extend_from_slice(&[NOT_FOUND]);
                Ok(())
            }
            Frame::NotDirectory => {
                out.extend_from_slice(&[NOT_DIRECTORY]);
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
            TREE => postcard::from_bytes(rest).map(Frame::Tree).map_err(FrameError::Tree),
            NOT_FOUND => Ok(Frame::NotFound),
            NOT_DIRECTORY => Ok(Frame::NotDirectory),
            FORBIDDEN => Ok(Frame::Forbidden),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A resources filetree response that could not be written.
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
            FrameEncodeError::Tree(error) => write!(f, "resources filetree tree did not serialize: {error}"),
            FrameEncodeError::Error(error) => write!(f, "resources filetree error did not serialize: {error}"),
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

/// A resources filetree response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's five.
    UnknownTag(u8),
    /// The tree did not decode.
    Tree(postcard::Error),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("resources filetree response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown resources filetree response frame tag {tag}"),
            FrameError::Tree(error) => write!(f, "resources filetree tree did not decode: {error}"),
            FrameError::Error(error) => write!(f, "resources filetree error did not parse: {error}"),
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
