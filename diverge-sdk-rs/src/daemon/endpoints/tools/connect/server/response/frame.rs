//! What a server's response frame carries for a connect.

use std::fmt;

use crate::shared::error::Error;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// A connect's answer: the tool is held under the name, the name is
/// in use, or a failure.
///
/// A connect is one question and one reply, so there is exactly one of
/// these per scope, before the finish that ends it. A payload leads
/// with one byte saying which — `0` for [`Connected`](Self::Connected),
/// `1` for [`InUse`](Self::InUse), `2` for
/// [`Forbidden`](Self::Forbidden), `3` for [`Error`](Self::Error) — and
/// only the error carries anything after it.
///
/// # The name in use is not a failure
///
/// [`InUse`](Self::InUse) is an ANSWER: a tool of the caller's exists
/// under that name already, and the daemon recorded nothing — the
/// caller uses the tool it has, or chooses another name, and nothing
/// is retried. An [`Error`](Self::Error) is the absence of an answer:
/// the tool could not be recorded, for whatever reason the daemon
/// knows, and the name is as it was.
///
/// # Connected carries nothing, and joins nothing
///
/// The name is the caller's handle from now on, and the caller chose
/// it. Nothing was joined: whether the daemon named is reachable and
/// exposes the tool to this caller is learned when an attached agent
/// is active, as the tool's `active` in a list and as the agent's tool
/// calls.
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
    /// The tool is held under the name. Tag `0`.
    Connected,
    /// A tool of the caller's exists under that name already;
    /// nothing was recorded. Tag `1`.
    InUse,
    /// The account the request is served for holds no grant allowing
    /// it; nothing changed. Tag `2`.
    Forbidden,
    /// A failure. Tag `3`.
    ///
    /// The tool is not held and will not be, and the name is
    /// unchanged. See
    /// [`shared::error::Error`](crate::shared::error::Error)
    /// for why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Connected`].
const CONNECTED: u8 = 0;

/// Tag for [`Frame::InUse`].
const IN_USE: u8 = 1;

/// Tag for [`Frame::Forbidden`].
const FORBIDDEN: u8 = 2;

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
            Frame::Connected => {
                out.extend_from_slice(&[CONNECTED]);
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
            CONNECTED => Ok(Frame::Connected),
            IN_USE => Ok(Frame::InUse),
            FORBIDDEN => Ok(Frame::Forbidden),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A connect response frame that could not be read.
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
            FrameError::Empty => f.write_str("tools connect response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown tools connect response frame tag {tag}"),
            FrameError::Error(error) => write!(f, "tools connect error did not parse: {error}"),
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
