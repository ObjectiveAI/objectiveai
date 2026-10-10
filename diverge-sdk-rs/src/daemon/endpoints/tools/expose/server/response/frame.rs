//! What a server's response frame carries for an expose.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::error::Error;

use super::Exposed;

/// An expose's answer: the exposure — where the container runs and
/// what admits one connection to it — no tool is the one named,
/// forbidden, or a failure.
///
/// An expose is a scope held open. Exactly one response leads it,
/// [`Exposed`](Self::Exposed), and nothing follows it for as long as
/// the tool's run lasts: the finish is the run ending, however it
/// ends, and it follows the client's cancel. Or the scope is exactly
/// one [`NotFound`](Self::NotFound), [`Forbidden`](Self::Forbidden) or
/// [`Error`](Self::Error), then the finish. A payload leads with one
/// byte saying which — `0` for `Exposed`, `1` for `NotFound`, `2` for
/// `Forbidden`, `3` for `Error` — and the rest is that variant's own
/// JSON: one [`Exposed`] for the first, the error for the last,
/// nothing for the others.
///
/// # A finish with nothing is an answer
///
/// The client cancelled before anything was sent, or the request was
/// not served: a scope that finishes with no response before it is
/// that, not a failure. [`NotFound`](Self::NotFound) is an ANSWER: the
/// daemon looked, and no tool of the caller's is the one named, and
/// nothing is retried. An [`Error`](Self::Error) is a failure: the
/// daemon could not start or expose the tool, for whatever reason it
/// knows — a dependency tool, which is read-only; a connected tool,
/// which is another daemon's and is exposed there; a container that
/// would not start.///
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
    /// The exposure: what joins the tool's container, which runs. Tag
    /// `0`, once, first.
    Exposed(Exposed),
    /// No tool of the caller's is the one named; nothing changed. Tag
    /// `1`.
    NotFound,
    /// The account the request is served for holds no grant allowing
    /// it; nothing changed. Tag `2`.
    Forbidden,
    /// A failure. Tag `3`.
    ///
    /// Nothing is exposed. See
    /// [`shared::error::Error`](crate::shared::error::Error) for why it
    /// says so little.
    Error(Error),
}

/// Tag for [`Frame::Exposed`].
const EXPOSED: u8 = 0;

/// Tag for [`Frame::NotFound`].
const NOT_FOUND: u8 = 1;

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
            Frame::Exposed(item) => {
                out.extend_from_slice(&[EXPOSED]);
                serde_json::to_writer(out, item)
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
            EXPOSED => serde_json::from_slice(rest).map(Frame::Exposed).map_err(FrameError::Exposed),
            NOT_FOUND => Ok(Frame::NotFound),
            FORBIDDEN => Ok(Frame::Forbidden),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// An expose response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's four.
    UnknownTag(u8),
    /// The exposure did not parse.
    Exposed(serde_json::Error),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("tools expose response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown tools expose response frame tag {tag}"),
            FrameError::Exposed(error) => write!(f, "tools expose exposure did not parse: {error}"),
            FrameError::Error(error) => write!(f, "tools expose error did not parse: {error}"),
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Exposed(error) | FrameError::Error(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
