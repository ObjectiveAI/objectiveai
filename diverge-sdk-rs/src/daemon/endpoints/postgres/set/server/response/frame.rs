//! What a server's response frame carries for a set.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::error::Error;

use crate::daemon::endpoints::postgres::Connection;

/// A set's answer: the mode is set, the database is in use, forbidden,
/// or a failure.
///
/// A set is one question and one reply, so there is exactly one of
/// these per scope, before the finish that ends it. A payload leads
/// with one byte saying which — `0` for [`Set`](Self::Set), `1` for
/// [`InUse`](Self::InUse), `2` for [`Forbidden`](Self::Forbidden), `3`
/// for [`Error`](Self::Error) — and the rest is that variant's own
/// JSON: nothing for the first and the third, the connections for the
/// second, the error for the last.
///
/// # Answers, and one failure
///
/// [`InUse`](Self::InUse) is an ANSWER: the daemon looked, container
/// connections are open through the database it serves now, and it will
/// not swap under them; the mode is as it was, nothing is retried, and
/// the connections it names are what the client ends before asking
/// again. An [`Error`](Self::Error) is the absence of an answer: the
/// daemon could not swap, for whatever reason it knows — a remote it
/// could not reach is one — and the mode is as it was.
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
    /// The daemon serves the mode the request gave, from now on. Tag
    /// `0`.
    Set,
    /// Container connections are open through the current database,
    /// these, oldest opened first; nothing changed. Tag `1`.
    InUse(Vec<Connection>),
    /// The account the request is served for holds no grant allowing
    /// it; nothing changed. Tag `2`.
    Forbidden,
    /// A failure. Tag `3`.
    ///
    /// Nothing changed. See
    /// [`shared::error::Error`](crate::shared::error::Error) for why it
    /// says so little.
    Error(Error),
}

/// Tag for [`Frame::Set`].
const SET: u8 = 0;

/// Tag for [`Frame::InUse`].
const IN_USE: u8 = 1;

/// Tag for [`Frame::Forbidden`].
const FORBIDDEN: u8 = 2;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 3;

/// A tag, then the variant's own JSON, if it has any.
impl Encode for Frame {
    /// The ordinary JSON failure. The bare answers cannot fail.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Set => {
                out.extend_from_slice(&[SET]);
                Ok(())
            }
            Frame::InUse(connections) => {
                out.extend_from_slice(&[IN_USE]);
                serde_json::to_writer(out, connections)
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
            SET => Ok(Frame::Set),
            IN_USE => serde_json::from_slice(rest).map(Frame::InUse).map_err(FrameError::InUse),
            FORBIDDEN => Ok(Frame::Forbidden),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A set response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's four.
    UnknownTag(u8),
    /// The connections did not parse.
    InUse(serde_json::Error),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("postgres set response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown postgres set response frame tag {tag}"),
            FrameError::InUse(error) => write!(f, "postgres set connections did not parse: {error}"),
            FrameError::Error(error) => write!(f, "postgres set error did not parse: {error}"),
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::InUse(error) | FrameError::Error(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
