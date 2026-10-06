//! What a server's response frame carries for a stat.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::error::Error;

use super::Stat;

/// A stat's answer: the two numbers, no volume is the one named, the
/// volume is held, forbidden, or a failure.
///
/// A stat is one question and one reply, so there is exactly one of
/// these per scope, before the finish that ends it. A payload leads
/// with one byte saying which — `0` for [`Stat`](Self::Stat), `1` for
/// [`NotFound`](Self::NotFound), `2` for [`Held`](Self::Held), `3` for
/// [`Forbidden`](Self::Forbidden), `4` for [`Error`](Self::Error) — and
/// the rest is that variant's own JSON: the stat for the first, nothing
/// for the bare answers, the error for the last.
///
/// # Answers, and one failure
///
/// [`NotFound`](Self::NotFound) and [`Held`](Self::Held) are ANSWERS:
/// the daemon looked, and no volume is the one named, or a running
/// container has it or a download, an upload or a transfer is on it,
/// and in either case nothing was walked and nothing is retried; a held
/// volume is asked about again once it is free. An
/// [`Error`](Self::Error) is the absence of an answer: the daemon could
/// not ask, or the provider could not finish the walk, in whichever's
/// words.
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
    /// The two numbers, as of the walk: see [`Stat`]. Tag `0`.
    Stat(Stat),
    /// No volume is the one named; nothing was walked. Tag `1`.
    NotFound,
    /// A running container has the volume, or another download, upload
    /// or transfer is on it; nothing was walked. Tag `2`.
    Held,
    /// The account the request is served for holds no grant allowing
    /// it; nothing changed. Tag `3`.
    Forbidden,
    /// A failure. Tag `4`.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Stat`].
const STAT: u8 = 0;

/// Tag for [`Frame::NotFound`].
const NOT_FOUND: u8 = 1;

/// Tag for [`Frame::Held`].
const HELD: u8 = 2;

/// Tag for [`Frame::Forbidden`].
const FORBIDDEN: u8 = 3;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 4;

/// A tag, then the variant's own JSON, if it has any.
impl Encode for Frame {
    /// The ordinary JSON failure. The bare answers cannot fail.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Stat(stat) => {
                out.extend_from_slice(&[STAT]);
                serde_json::to_writer(out, stat)
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
    /// Four ways to fail, and each names which half failed.
    type Error = FrameError;

    // Spelled out for the same reason as `encode` above.
    fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            STAT => serde_json::from_slice(rest).map(Frame::Stat).map_err(FrameError::Stat),
            NOT_FOUND => Ok(Frame::NotFound),
            HELD => Ok(Frame::Held),
            FORBIDDEN => Ok(Frame::Forbidden),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A stat response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's five.
    UnknownTag(u8),
    /// The stat did not parse.
    Stat(serde_json::Error),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("volumes stat response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown volumes stat response frame tag {tag}"),
            FrameError::Stat(error) => write!(f, "volumes stat stat did not parse: {error}"),
            FrameError::Error(error) => write!(f, "volumes stat error did not parse: {error}"),
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Stat(error) | FrameError::Error(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
