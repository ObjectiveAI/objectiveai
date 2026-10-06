//! What a server's response frame carries for an upload.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::error::Error;

/// An upload's answer: the files are in place, no volume is the one
/// named, the volume is held, forbidden, or a failure.
///
/// An upload is one question and one reply, sent once every content
/// channel has finished, so there is exactly one of these per scope,
/// before the finish that ends it. A payload leads
/// with one byte saying which — `0` for [`Uploaded`](Self::Uploaded),
/// `1` for [`NotFound`](Self::NotFound), `2` for [`Held`](Self::Held),
/// `3` for [`Forbidden`](Self::Forbidden), `4` for
/// [`Error`](Self::Error) — and only the error carries anything after
/// it.
///
/// # Answers, and one failure
///
/// [`NotFound`](Self::NotFound) and [`Held`](Self::Held) are ANSWERS:
/// the daemon looked, and no volume is the one named, or a running
/// container has it or another download, upload or transfer is on it,
/// and in either case nothing was asked for and nothing landed. An
/// [`Error`](Self::Error) is the absence of an answer: a content
/// channel ended in an error, a path was never finished, the provider
/// could not take the file, whatever the daemon knows; a file is at its
/// path whole or as it was, never the half between, and files that
/// landed before the failure stay.
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
    /// Every file is in place. Tag `0`.
    Uploaded,
    /// No volume is the one named; nothing landed. Tag `1`.
    NotFound,
    /// A running container has the volume, or another download, upload
    /// or transfer is on it; nothing landed. Tag `2`.
    Held,
    /// The account the request is served for holds no grant allowing
    /// it; nothing changed. Tag `3`.
    Forbidden,
    /// A failure. Tag `4`.
    ///
    /// Nothing changed. See
    /// [`shared::error::Error`](crate::shared::error::Error) for why it
    /// says so little.
    Error(Error),
}

/// Tag for [`Frame::Uploaded`].
const UPLOADED: u8 = 0;

/// Tag for [`Frame::NotFound`].
const NOT_FOUND: u8 = 1;

/// Tag for [`Frame::Held`].
const HELD: u8 = 2;

/// Tag for [`Frame::Forbidden`].
const FORBIDDEN: u8 = 3;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 4;

/// A tag, and — for the error alone — that variant's own JSON.
impl Encode for Frame {
    /// The ordinary JSON failure. The bare answers cannot fail.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Uploaded => {
                out.extend_from_slice(&[UPLOADED]);
                Ok(())
            }
            Frame::NotFound => {
                out.extend_from_slice(&[NOT_FOUND]);
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
            UPLOADED => Ok(Frame::Uploaded),
            NOT_FOUND => Ok(Frame::NotFound),
            HELD => Ok(Frame::Held),
            FORBIDDEN => Ok(Frame::Forbidden),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// An upload response frame that could not be read.
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
            FrameError::Empty => f.write_str("volumes upload response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown volumes upload response frame tag {tag}"),
            FrameError::Error(error) => write!(f, "volumes upload error did not parse: {error}"),
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
