//! What a server's response frame carries for an edit.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::error::Error;

/// An edit's answer: the tool is as the request states, no tool is the
/// one named, the tool is active, the tool is connected, the name is
/// another's, the account is none the daemon has, or a failure.
///
/// An edit is one question and one reply, so there is exactly one of
/// these per scope, before the finish that ends it. A payload leads
/// with one byte saying which — `0` for [`Edited`](Self::Edited), `1`
/// for [`NotFound`](Self::NotFound), `2` for [`Active`](Self::Active),
/// `3` for [`NotOwned`](Self::NotOwned), `4` for
/// [`InUse`](Self::InUse), `5` for [`NoAccount`](Self::NoAccount), `6`
/// for [`Error`](Self::Error) — and only the error carries anything
/// after it.
///
/// # Answers, and one failure
///
/// [`NotFound`](Self::NotFound), [`Active`](Self::Active),
/// [`NotOwned`](Self::NotOwned), [`InUse`](Self::InUse) and
/// [`NoAccount`](Self::NoAccount) are ANSWERS: the daemon looked, and
/// no tool of the caller's is the one named, or one is and it is active
/// while the request names a mount, or it is a connected tool and the
/// request names a mount or an account, or the name requested is
/// another tool's, or the account requested is none the daemon has, and
/// in each case nothing changed and nothing is retried. An
/// [`Error`](Self::Error) is the absence of an answer: the daemon could
/// not make the change, for whatever reason it knows, and the tool is
/// as it was. The request is applied whole or not at all.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The tool is as the request states. Tag `0`.
    Edited,
    /// No tool of the caller's is the one named; nothing changed. Tag
    /// `1`.
    NotFound,
    /// The tool is active — its container running, or its connect scope
    /// held — and the request names a mount; nothing changed. Tag `2`.
    Active,
    /// The tool is connected, somebody else's container, and the
    /// request names a mount or an account, which it has none of this
    /// caller's; nothing changed. Tag `3`.
    NotOwned,
    /// The name requested is another tool's; nothing changed. Tag `4`.
    InUse,
    /// The account requested is none the daemon has; nothing changed.
    /// Tag `5`.
    NoAccount,
    /// A failure. Tag `6`.
    ///
    /// Nothing changed. See
    /// [`shared::error::Error`](crate::shared::error::Error) for why it
    /// says so little.
    Error(Error),
}

/// Tag for [`Frame::Edited`].
const EDITED: u8 = 0;

/// Tag for [`Frame::NotFound`].
const NOT_FOUND: u8 = 1;

/// Tag for [`Frame::Active`].
const ACTIVE: u8 = 2;

/// Tag for [`Frame::NotOwned`].
const NOT_OWNED: u8 = 3;

/// Tag for [`Frame::InUse`].
const IN_USE: u8 = 4;

/// Tag for [`Frame::NoAccount`].
const NO_ACCOUNT: u8 = 5;

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
            Frame::Edited => {
                out.extend_from_slice(&[EDITED]);
                Ok(())
            }
            Frame::NotFound => {
                out.extend_from_slice(&[NOT_FOUND]);
                Ok(())
            }
            Frame::Active => {
                out.extend_from_slice(&[ACTIVE]);
                Ok(())
            }
            Frame::NotOwned => {
                out.extend_from_slice(&[NOT_OWNED]);
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
            EDITED => Ok(Frame::Edited),
            NOT_FOUND => Ok(Frame::NotFound),
            ACTIVE => Ok(Frame::Active),
            NOT_OWNED => Ok(Frame::NotOwned),
            IN_USE => Ok(Frame::InUse),
            NO_ACCOUNT => Ok(Frame::NoAccount),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// An edit response frame that could not be read.
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
            FrameError::Empty => f.write_str("tools edit response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown tools edit response frame tag {tag}"),
            FrameError::Error(error) => write!(f, "tools edit error did not parse: {error}"),
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
