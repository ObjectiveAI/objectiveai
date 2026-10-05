//! What a server's response frame carries for a route delete.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::error::Error;

/// A route delete's answer: the route is gone, the position has none,
/// the route is in use, or a failure.
///
/// A route delete is one question and one reply, so there is exactly
/// one of these per scope, before the finish that ends it. A payload
/// leads with one byte saying which — `0` for
/// [`Deleted`](Self::Deleted), `1` for [`NotFound`](Self::NotFound),
/// `2` for [`InUse`](Self::InUse), `3` for
/// [`Forbidden`](Self::Forbidden), `4` for [`Error`](Self::Error) — and
/// only the error carries anything after it.
///
/// # Answers, and one failure
///
/// [`NotFound`](Self::NotFound) and [`InUse`](Self::InUse) are ANSWERS:
/// the daemon looked, and either the position has no route, or it has
/// and an active container is served through it, and in either case
/// nothing changed and nothing is retried. An [`Error`](Self::Error) is
/// the absence of an answer: the daemon could not take the route up,
/// for whatever reason it knows.
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
    /// The route is gone: the position is the deployer's again. Tag
    /// `0`.
    Deleted,
    /// The position has no route; nothing changed. Tag `1`.
    NotFound,
    /// An active container is served through the route, which was left
    /// as it is; nothing changed. Tag `2`.
    InUse,
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

/// Tag for [`Frame::Deleted`].
const DELETED: u8 = 0;

/// Tag for [`Frame::NotFound`].
const NOT_FOUND: u8 = 1;

/// Tag for [`Frame::InUse`].
const IN_USE: u8 = 2;

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
            Frame::Deleted => {
                out.extend_from_slice(&[DELETED]);
                Ok(())
            }
            Frame::NotFound => {
                out.extend_from_slice(&[NOT_FOUND]);
                Ok(())
            }
            Frame::InUse => {
                out.extend_from_slice(&[IN_USE]);
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
            DELETED => Ok(Frame::Deleted),
            NOT_FOUND => Ok(Frame::NotFound),
            IN_USE => Ok(Frame::InUse),
            FORBIDDEN => Ok(Frame::Forbidden),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A route delete response frame that could not be read.
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
            FrameError::Empty => f.write_str("tools routes delete response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown tools routes delete response frame tag {tag}"),
            FrameError::Error(error) => write!(f, "tools routes delete error did not parse: {error}"),
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
