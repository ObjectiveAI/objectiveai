//! What a server's response frame carries for a create.

use serde::Serialize;
use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::error::Error;

/// A create's answer: the volume exists, no provider is the one named,
/// the name is taken, the provider has no room, the provider keeps no
/// such volume, forbidden, or a failure.
///
/// A create is one question and one reply, so there is exactly one of
/// these per scope, before the finish that ends it. A payload leads
/// with one byte saying which — `0` for [`Created`](Self::Created), `1`
/// for [`NoProvider`](Self::NoProvider), `2` for
/// [`Exists`](Self::Exists), `3` for
/// [`InsufficientCapacity`](Self::InsufficientCapacity), `4` for
/// [`Unsupported`](Self::Unsupported), `5` for
/// [`Forbidden`](Self::Forbidden), `6` for [`Error`](Self::Error) — and
/// only the error carries anything after it.
///
/// # Answers, and one failure
///
/// [`NoProvider`](Self::NoProvider), [`Exists`](Self::Exists),
/// [`InsufficientCapacity`](Self::InsufficientCapacity) and
/// [`Unsupported`](Self::Unsupported) are ANSWERS: the daemon looked,
/// and no provider is the one named, or the name is a volume's on
/// that provider already, or the provider cannot reserve that many
/// bytes, or the provider's capabilities keep no volume at all or
/// none of the mode asked — which the daemon knows from the
/// provider's `info` and asks no further — and in each case nothing
/// exists and nothing is retried: a size the provider has no room for
/// is one to ask smaller, a mode it does not keep one to ask another
/// provider for.
/// An [`Error`](Self::Error) is the absence of an answer: the daemon
/// could not ask, or the provider answered an error, in whichever's
/// words, and nothing partial exists.
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
///
/// # As JSON
///
/// For a client that prints what the daemon answered: the variant,
/// snake case, over its payload — `{"found":…}`, `{"error":…}` — and
/// a bare variant the string alone — `"not_found"`. The wire is the
/// tag byte and the payload, never this.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Frame {
    /// The volume exists, and a list shows it. Tag `0`.
    Created,
    /// No provider is the one named; nothing exists. Tag `1`.
    NoProvider,
    /// The name is a volume's on that provider already; nothing
    /// changed. Tag `2`.
    Exists,
    /// The provider cannot reserve that many bytes; nothing exists. Tag
    /// `3`.
    InsufficientCapacity,
    /// The provider's capabilities keep no volume, or none of the mode
    /// asked; nothing exists, and the provider was not asked. Tag `4`.
    Unsupported,
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

/// Tag for [`Frame::Created`].
const CREATED: u8 = 0;

/// Tag for [`Frame::NoProvider`].
const NO_PROVIDER: u8 = 1;

/// Tag for [`Frame::Exists`].
const EXISTS: u8 = 2;

/// Tag for [`Frame::InsufficientCapacity`].
const INSUFFICIENT_CAPACITY: u8 = 3;

/// Tag for [`Frame::Unsupported`].
const UNSUPPORTED: u8 = 4;

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
            Frame::Created => {
                out.extend_from_slice(&[CREATED]);
                Ok(())
            }
            Frame::NoProvider => {
                out.extend_from_slice(&[NO_PROVIDER]);
                Ok(())
            }
            Frame::Exists => {
                out.extend_from_slice(&[EXISTS]);
                Ok(())
            }
            Frame::InsufficientCapacity => {
                out.extend_from_slice(&[INSUFFICIENT_CAPACITY]);
                Ok(())
            }
            Frame::Unsupported => {
                out.extend_from_slice(&[UNSUPPORTED]);
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
            CREATED => Ok(Frame::Created),
            NO_PROVIDER => Ok(Frame::NoProvider),
            EXISTS => Ok(Frame::Exists),
            INSUFFICIENT_CAPACITY => Ok(Frame::InsufficientCapacity),
            UNSUPPORTED => Ok(Frame::Unsupported),
            FORBIDDEN => Ok(Frame::Forbidden),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A create response frame that could not be read.
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
            FrameError::Empty => f.write_str("volumes create response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown volumes create response frame tag {tag}"),
            FrameError::Error(error) => write!(f, "volumes create error did not parse: {error}"),
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
