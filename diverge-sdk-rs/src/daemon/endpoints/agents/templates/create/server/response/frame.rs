//! What a server's response frame carries for a templates create.

use std::fmt;

use crate::shared::error::Error;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// A create's answer: the template is made, under this id; a template
/// of the same hash was made before, and this is its id; or a
/// failure.
///
/// A create is one question and one reply, so there is exactly one of
/// these per scope, before the finish that ends it. A payload leads
/// with one byte saying which — `0` for [`Created`](Self::Created), `1`
/// for [`Exists`](Self::Exists), `2` for
/// [`Forbidden`](Self::Forbidden), `3` for [`Error`](Self::Error) — and
/// the rest is that variant's own JSON: the id as a JSON string for the
/// first two, the error for the third.
///
/// # Exists is not a failure
///
/// [`Exists`](Self::Exists) is an ANSWER: the caller holds a template
/// of exactly this hash already, and the daemon made nothing — the id
/// answered is the one the caller has, and nothing is retried. An
/// [`Error`](Self::Error) is the absence of an answer: the template
/// could not be made, for whatever reason the daemon knows, and
/// nothing is held.
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
    /// The template is made, and this is its id. Tag `0`.
    Created(String),
    /// A template of the same hash was made before, and this is its
    /// id; nothing was made. Tag `1`.
    Exists(String),
    /// The account the request is served for holds no grant allowing
    /// it; nothing changed. Tag `2`.
    Forbidden,
    /// A failure. Tag `3`.
    ///
    /// Nothing is held. See
    /// [`shared::error::Error`](crate::shared::error::Error)
    /// for why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Created`].
const CREATED: u8 = 0;

/// Tag for [`Frame::Exists`].
const EXISTS: u8 = 1;

/// Tag for [`Frame::Forbidden`].
const FORBIDDEN: u8 = 2;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 3;

/// A tag, then the variant's own JSON.
impl Encode for Frame {
    /// The ordinary JSON failure.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Created(id) => {
                out.extend_from_slice(&[CREATED]);
                serde_json::to_writer(out, id)
            }
            Frame::Exists(id) => {
                out.extend_from_slice(&[EXISTS]);
                serde_json::to_writer(out, id)
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
            CREATED => serde_json::from_slice(rest).map(Frame::Created).map_err(FrameError::Id),
            EXISTS => serde_json::from_slice(rest).map(Frame::Exists).map_err(FrameError::Id),
            FORBIDDEN => Ok(Frame::Forbidden),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A templates create response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's four.
    UnknownTag(u8),
    /// The id did not parse as a JSON string.
    Id(serde_json::Error),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("agents templates create response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown agents templates create response frame tag {tag}"),
            FrameError::Id(error) => write!(f, "agents templates create id did not parse: {error}"),
            FrameError::Error(error) => write!(f, "agents templates create error did not parse: {error}"),
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Id(error) | FrameError::Error(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
