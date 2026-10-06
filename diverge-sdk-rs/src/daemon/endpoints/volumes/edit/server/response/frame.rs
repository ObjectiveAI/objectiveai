//! What a server's response frame carries for an edit.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::error::Error;

/// An edit's answer: the volume is as the request states, no volume is
/// the one named, the provider has no room, the volume holds more than
/// that, the volume is held, forbidden, or a failure.
///
/// An edit is one question and one reply, so there is exactly one of
/// these per scope, before the finish that ends it. A payload leads
/// with one byte saying which — `0` for [`Edited`](Self::Edited), `1`
/// for [`NotFound`](Self::NotFound), `2` for
/// [`InsufficientCapacity`](Self::InsufficientCapacity), `3` for
/// [`ContentTooLarge`](Self::ContentTooLarge), `4` for
/// [`Held`](Self::Held), `5` for [`Forbidden`](Self::Forbidden), `6`
/// for [`Error`](Self::Error) — and only the error carries anything
/// after it.
///
/// # Answers, and one failure
///
/// [`NotFound`](Self::NotFound),
/// [`InsufficientCapacity`](Self::InsufficientCapacity),
/// [`ContentTooLarge`](Self::ContentTooLarge) and [`Held`](Self::Held)
/// are ANSWERS: the daemon looked, and no volume is the one named, or
/// the provider cannot reserve that many bytes, or the volume holds
/// more than the size asked for, or a running container has it or a
/// download, an upload or a transfer is on it, and in each case nothing
/// changed and nothing is retried — a size to ask smaller, a volume to
/// empty first, a volume to ask about again once it is free. An
/// [`Error`](Self::Error) is the absence of an answer: the daemon could
/// not ask, or the provider answered an error, in whichever's words,
/// and the volume is as it was. The request is applied whole or not at
/// all.
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
    /// The volume is as the request states. Tag `0`.
    Edited,
    /// No volume is the one named; nothing changed. Tag `1`.
    NotFound,
    /// The provider cannot reserve that many bytes; nothing changed.
    /// Tag `2`.
    InsufficientCapacity,
    /// The volume holds more than the size asked for, so it cannot be
    /// shrunk to it; nothing changed. Tag `3`.
    ContentTooLarge,
    /// A running container has the volume, or another download, upload
    /// or transfer is on it; nothing changed. Tag `4`.
    Held,
    /// The account the request is served for holds no grant allowing
    /// it; nothing changed. Tag `5`.
    Forbidden,
    /// A failure. Tag `6`.
    ///
    /// Nothing changed. See
    /// [`shared::error::Error`](crate::shared::error::Error) for why it
    /// says so little.
    Error(Error),
}

/// Tag for [`Frame::Edited`].
const EDITED: u8 = 0;

/// Tag for [`Frame::NotFound`].
const NOT_FOUND: u8 = 1;

/// Tag for [`Frame::InsufficientCapacity`].
const INSUFFICIENT_CAPACITY: u8 = 2;

/// Tag for [`Frame::ContentTooLarge`].
const CONTENT_TOO_LARGE: u8 = 3;

/// Tag for [`Frame::Held`].
const HELD: u8 = 4;

/// Tag for [`Frame::Forbidden`].
const FORBIDDEN: u8 = 5;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 6;

/// A tag, and — for the error alone — that variant's own JSON.
impl Encode for Frame {
    /// The ordinary JSON failure. The bare answers cannot fail.
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
            Frame::InsufficientCapacity => {
                out.extend_from_slice(&[INSUFFICIENT_CAPACITY]);
                Ok(())
            }
            Frame::ContentTooLarge => {
                out.extend_from_slice(&[CONTENT_TOO_LARGE]);
                Ok(())
            }
            Frame::Held => {
                out.extend_from_slice(&[HELD]);
                Ok(())
            }
            Frame::Forbidden => {
                out.extend_from_slice(&[FORBIDDEN]);
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
            INSUFFICIENT_CAPACITY => Ok(Frame::InsufficientCapacity),
            CONTENT_TOO_LARGE => Ok(Frame::ContentTooLarge),
            HELD => Ok(Frame::Held),
            FORBIDDEN => Ok(Frame::Forbidden),
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
    /// A tag that is none of this frame's seven.
    UnknownTag(u8),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("volumes edit response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown volumes edit response frame tag {tag}"),
            FrameError::Error(error) => write!(f, "volumes edit error did not parse: {error}"),
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
