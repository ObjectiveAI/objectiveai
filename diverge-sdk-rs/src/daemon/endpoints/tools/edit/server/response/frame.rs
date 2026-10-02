//! What a server's response frame carries for an edit.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::error::Error;

/// An edit's answer: the tool has the mounts stated, no tool has the
/// name, the tool is active, the tool is somebody else's, or a
/// failure.
///
/// An edit is one question and one reply, so there is exactly one of
/// these per scope, before the finish that ends it. A payload leads
/// with one byte saying which — `0` for [`Edited`](Self::Edited),
/// `1` for [`NotFound`](Self::NotFound), `2` for
/// [`Active`](Self::Active), `3` for [`NotOwned`](Self::NotOwned),
/// `4` for [`Error`](Self::Error) — and
/// only the error carries anything after it.
///
/// # Four answers, one failure
///
/// [`NotFound`](Self::NotFound), [`Active`](Self::Active) and
/// [`NotOwned`](Self::NotOwned) are ANSWERS: the daemon looked, and
/// either no tool of the caller's is the one named, or one does and its
/// container is running, or one does and it is a
/// [`connect`](crate::daemon::endpoints::tools::connect)ed tool with
/// no mounts of this caller's, and in every case nothing was changed
/// and nothing is retried — the caller has the wrong name, waits for
/// the container to stop and asks again, or asked the wrong kind of
/// tool. An [`Error`](Self::Error) is the absence of an answer:
/// the daemon could not change the tool's mounts, for whatever reason
/// it knows, and the tool mounts what it mounted.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The tool mounts what the request stated, and its next run
    /// sees them. Tag `0`.
    Edited,
    /// No tool of the caller's is the one named; nothing was changed.
    /// Tag `1`.
    NotFound,
    /// The tool is active — its container is running — and was left
    /// as it is; nothing was changed. Tag `2`.
    Active,
    /// The tool is a connected one, somebody else's container with no
    /// mounts of this caller's; nothing was changed. Tag `3`.
    NotOwned,
    /// A failure. Tag `4`.
    ///
    /// The tool mounts what it mounted. See
    /// [`shared::error::Error`](crate::shared::error::Error)
    /// for why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Edited`].
const EDITED: u8 = 0;

/// Tag for [`Frame::NotFound`].
const NOT_FOUND: u8 = 1;

/// Tag for [`Frame::Active`].
const ACTIVE: u8 = 2;

/// Tag for [`Frame::NotOwned`].
const NOT_OWNED: u8 = 3;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 4;

/// A tag, and — for the error alone — that variant's own JSON.
impl Encode for Frame {
    /// The ordinary JSON failure. The three bare answers cannot fail.
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
            Frame::Active => {
                out.extend_from_slice(&[ACTIVE]);
                Ok(())
            }
            Frame::NotOwned => {
                out.extend_from_slice(&[NOT_OWNED]);
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
    /// Three ways to fail, and only one of them is JSON.
    type Error = FrameError;

    // Spelled out for the same reason as `encode` above.
    fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            EDITED => Ok(Frame::Edited),
            NOT_FOUND => Ok(Frame::NotFound),
            ACTIVE => Ok(Frame::Active),
            NOT_OWNED => Ok(Frame::NotOwned),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// An edit response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's five.
    UnknownTag(u8),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("tools edit response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown tools edit response frame tag {tag}"),
            FrameError::Error(error) => write!(f, "tools edit error did not parse: {error}"),
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
