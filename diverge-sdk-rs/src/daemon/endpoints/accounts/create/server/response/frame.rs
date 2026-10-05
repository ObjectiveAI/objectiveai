//! What a server's response frame carries for a create.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::error::Error;

/// A create's answer: the account exists, one like it exists already,
/// no such role, no such hook resource, forbidden, or a failure.
///
/// A create is one question and one reply, so there is exactly one of
/// these per scope, before the finish that ends it. A payload leads
/// with one byte saying which — `0` for [`Created`](Self::Created), `1`
/// for [`Exists`](Self::Exists), `2` for [`NoRole`](Self::NoRole), `3`
/// for [`NoResource`](Self::NoResource), `4` for
/// [`Forbidden`](Self::Forbidden), `5` for [`Error`](Self::Error) — and
/// only the error carries anything after it.
///
/// # Answers, and one failure
///
/// [`Exists`](Self::Exists), [`NoRole`](Self::NoRole) and
/// [`NoResource`](Self::NoResource) are ANSWERS: the daemon looked, and
/// the name, the identity or the resource is an account's already, or a
/// role named is none it has, or the hook names a resource the caller
/// does not hold, or a file, or a directory with no `hook.yaml` at its
/// root, and in each case nothing was created and nothing is retried.
/// An [`Error`](Self::Error) is the absence of an answer: the daemon
/// could not create the account, for whatever reason it knows.
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
    /// The account exists, with its roles. Tag `0`.
    Created,
    /// The name, the identity or the resource is an account's already;
    /// nothing was created. Tag `1`.
    Exists,
    /// A role named is none the daemon has; nothing was created. Tag
    /// `2`.
    NoRole,
    /// The hook names no resource the caller holds, or one that is not
    /// a directory with `hook.yaml` at its root; nothing was created.
    /// Tag `3`.
    NoResource,
    /// The account the request is served for holds no grant allowing
    /// it; nothing changed. Tag `4`.
    Forbidden,
    /// A failure. Tag `5`.
    ///
    /// Nothing changed. See
    /// [`shared::error::Error`](crate::shared::error::Error) for why it
    /// says so little.
    Error(Error),
}

/// Tag for [`Frame::Created`].
const CREATED: u8 = 0;

/// Tag for [`Frame::Exists`].
const EXISTS: u8 = 1;

/// Tag for [`Frame::NoRole`].
const NO_ROLE: u8 = 2;

/// Tag for [`Frame::NoResource`].
const NO_RESOURCE: u8 = 3;

/// Tag for [`Frame::Forbidden`].
const FORBIDDEN: u8 = 4;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 5;

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
            Frame::Exists => {
                out.extend_from_slice(&[EXISTS]);
                Ok(())
            }
            Frame::NoRole => {
                out.extend_from_slice(&[NO_ROLE]);
                Ok(())
            }
            Frame::NoResource => {
                out.extend_from_slice(&[NO_RESOURCE]);
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
            EXISTS => Ok(Frame::Exists),
            NO_ROLE => Ok(Frame::NoRole),
            NO_RESOURCE => Ok(Frame::NoResource),
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
    /// A tag that is none of this frame's six.
    UnknownTag(u8),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("accounts create response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown accounts create response frame tag {tag}"),
            FrameError::Error(error) => write!(f, "accounts create error did not parse: {error}"),
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
