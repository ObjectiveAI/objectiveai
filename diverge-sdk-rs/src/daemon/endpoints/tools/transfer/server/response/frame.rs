//! What a server's response frame carries for a transfer.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::error::Error;

/// A transfer's answer: it landed, no tool is the one named, no agent,
/// tool or volume is the destination's, a volume held, forbidden, or a
/// failure.
///
/// A transfer is one question and one reply, sent once everything has
/// landed, so there is exactly one of these per scope, before the
/// finish that ends it. A payload leads with one byte saying which —
/// `0` for [`Transferred`](Self::Transferred), `1` for
/// [`NotFound`](Self::NotFound), `2` for
/// [`NoDestination`](Self::NoDestination), `3` for
/// [`Held`](Self::Held), `4` for [`Forbidden`](Self::Forbidden), `5`
/// for [`Error`](Self::Error) — and the rest is that variant's own
/// JSON: nothing for the first five; the error for the last.
///
/// # Answers, and one failure
///
/// [`NotFound`](Self::NotFound), [`NoDestination`](Self::NoDestination)
/// and [`Held`](Self::Held) are ANSWERS: the daemon looked, and no tool
/// is the one named or nothing is at the path, or the destination names
/// an agent, a tool or a volume that is none, or a volume at either end
/// is held — a running container has it, or another download, upload or
/// transfer is on it — and in each case nothing landed and nothing is
/// retried. An [`Error`](Self::Error) is the absence of an answer: the
/// daemon could not copy, or could not finish copying, for whatever
/// reason it knows; a file is at the destination whole or as it was,
/// never the half between, and files that landed before the failure
/// stay.
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
    /// Everything landed. Tag `0`.
    Transferred,
    /// No tool is the one named, or nothing is at the path; nothing
    /// landed. Tag `1`.
    NotFound,
    /// The destination names an agent, a tool or a volume that is none;
    /// nothing landed. Tag `2`.
    NoDestination,
    /// A volume at either end is held: a running container has it, or
    /// another download, upload or transfer is on it; nothing landed.
    /// Tag `3`.
    Held,
    /// The account the request is served for holds no grant allowing
    /// it; nothing landed. Tag `4`.
    Forbidden,
    /// A failure. Tag `5`.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Transferred`].
const TRANSFERRED: u8 = 0;

/// Tag for [`Frame::NotFound`].
const NOT_FOUND: u8 = 1;

/// Tag for [`Frame::NoDestination`].
const NO_DESTINATION: u8 = 2;

/// Tag for [`Frame::Held`].
const HELD: u8 = 3;

/// Tag for [`Frame::Forbidden`].
const FORBIDDEN: u8 = 4;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 5;

/// A tag, then the variant's own JSON, if it has any.
impl Encode for Frame {
    /// The ordinary JSON failure. The bare answers cannot fail.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Transferred => {
                out.extend_from_slice(&[TRANSFERRED]);
                Ok(())
            }
            Frame::NotFound => {
                out.extend_from_slice(&[NOT_FOUND]);
                Ok(())
            }
            Frame::NoDestination => {
                out.extend_from_slice(&[NO_DESTINATION]);
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
    /// Three ways to fail, and each names which half failed.
    type Error = FrameError;

    // Spelled out for the same reason as `encode` above.
    fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            TRANSFERRED => Ok(Frame::Transferred),
            NOT_FOUND => Ok(Frame::NotFound),
            NO_DESTINATION => Ok(Frame::NoDestination),
            HELD => Ok(Frame::Held),
            FORBIDDEN => Ok(Frame::Forbidden),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A transfer response frame that could not be read.
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
            FrameError::Empty => f.write_str("tools transfer response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown tools transfer response frame tag {tag}"),
            FrameError::Error(error) => write!(f, "tools transfer error did not parse: {error}"),
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
