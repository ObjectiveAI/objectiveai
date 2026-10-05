//! What a server's response frame carries for a create.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::error::Error;

/// A create's answer: the tool exists under the name, the name is in
/// use, the account is none the daemon has, or a failure.
///
/// A create is one question and one reply, so there is exactly one of
/// these per scope, before the finish that ends it. A payload leads
/// with one byte saying which — `0` for [`Created`](Self::Created), `1`
/// for [`InUse`](Self::InUse), `2` for [`NoAccount`](Self::NoAccount),
/// `3` for [`Error`](Self::Error) — and only the error carries anything
/// after it.
///
/// # Answers, and one failure
///
/// [`InUse`](Self::InUse) and [`NoAccount`](Self::NoAccount) are
/// ANSWERS: the daemon looked, and either the name is a tool's of the
/// caller's already, or the account named is none it has, and in either
/// case nothing was created and nothing is retried. An
/// [`Error`](Self::Error) is the absence of an answer: the daemon could
/// not create the tool, for whatever reason it knows, and nothing is
/// held.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The tool exists, under the name if one was given, and is reached
    /// by its template and index from now on. Tag `0`.
    Created,
    /// The name is a tool's of the caller's already; nothing was
    /// created. Tag `1`.
    InUse,
    /// The account named is none the daemon has; nothing was created.
    /// Tag `2`.
    NoAccount,
    /// A failure. Tag `3`.
    ///
    /// Nothing changed. See
    /// [`shared::error::Error`](crate::shared::error::Error) for why it
    /// says so little.
    Error(Error),
}

/// Tag for [`Frame::Created`].
const CREATED: u8 = 0;

/// Tag for [`Frame::InUse`].
const IN_USE: u8 = 1;

/// Tag for [`Frame::NoAccount`].
const NO_ACCOUNT: u8 = 2;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 3;

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
            Frame::InUse => {
                out.extend_from_slice(&[IN_USE]);
                Ok(())
            }
            Frame::NoAccount => {
                out.extend_from_slice(&[NO_ACCOUNT]);
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
            IN_USE => Ok(Frame::InUse),
            NO_ACCOUNT => Ok(Frame::NoAccount),
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
    /// A tag that is none of this frame's four.
    UnknownTag(u8),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("tools create response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown tools create response frame tag {tag}"),
            FrameError::Error(error) => write!(f, "tools create error did not parse: {error}"),
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
