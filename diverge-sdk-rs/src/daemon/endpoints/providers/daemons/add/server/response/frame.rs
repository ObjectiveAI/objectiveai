//! What a server's response frame carries for an add.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::error::Error;

/// An add's answer: the daemon is added, the name is a daemon's
/// already, a link names a provider the caller has no record of, or a
/// failure.
///
/// An add is one question and one reply, so there is exactly one of
/// these per scope, before the finish that ends it. A payload leads
/// with one byte saying which — `0` for [`Added`](Self::Added), `1` for
/// [`Exists`](Self::Exists), `2` for [`NoProvider`](Self::NoProvider),
/// `3` for [`Forbidden`](Self::Forbidden), `4` for
/// [`Error`](Self::Error) — and only the error carries anything after
/// it.
///
/// # Answers, and one failure
///
/// [`Exists`](Self::Exists) and [`NoProvider`](Self::NoProvider) are
/// ANSWERS: the caller has a daemon under that name already, or a
/// link's provider is none of the caller's, and in either case nothing
/// changed and nothing is retried — the caller has it, or edits it; or
/// adds the provider first. An [`Error`](Self::Error) is the absence of
/// an answer: the daemon could not add it, for whatever reason it
/// knows.///
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
    /// The daemon is added, under its name. Tag `0`.
    Added,
    /// The name is a daemon's of the caller's already; nothing changed.
    /// Tag `1`.
    Exists,
    /// A link names a provider the caller has no record of, outgoing or
    /// incoming; nothing changed. Tag `2`.
    NoProvider,
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

/// Tag for [`Frame::Added`].
const ADDED: u8 = 0;

/// Tag for [`Frame::Exists`].
const EXISTS: u8 = 1;

/// Tag for [`Frame::NoProvider`].
const NO_PROVIDER: u8 = 2;

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
            Frame::Added => {
                out.extend_from_slice(&[ADDED]);
                Ok(())
            }
            Frame::Exists => {
                out.extend_from_slice(&[EXISTS]);
                Ok(())
            }
            Frame::NoProvider => {
                out.extend_from_slice(&[NO_PROVIDER]);
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
            ADDED => Ok(Frame::Added),
            EXISTS => Ok(Frame::Exists),
            NO_PROVIDER => Ok(Frame::NoProvider),
            FORBIDDEN => Ok(Frame::Forbidden),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// An add response frame that could not be read.
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
            FrameError::Empty => f.write_str("providers daemons add response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown providers daemons add response frame tag {tag}"),
            FrameError::Error(error) => write!(f, "providers daemons add error did not parse: {error}"),
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
