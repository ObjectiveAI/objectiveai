//! What a server's response frame carries for a message.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::error::Error;

/// A message's answer: the agent took it, the client cancelled it
/// first, or a failure.
///
/// A message is one question and one reply, so there is exactly one
/// of these per scope, before the finish that ends it — but the
/// reply comes when the fate is known, which may be long after the
/// ask: a message queued behind a running loop waits for a seam of
/// the agent's choosing, and nothing times it out. A payload leads
/// with one byte saying which — `0` for
/// [`Delivered`](Self::Delivered), `1` for
/// [`Cancelled`](Self::Cancelled), `2` for [`Error`](Self::Error) —
/// and only the error carries anything after it.
///
/// # Cancelled is not a failure
///
/// [`Cancelled`](Self::Cancelled) is an ANSWER: the client opened a
/// [cancel](crate::daemon::endpoints::agents::message::client::channel_request::Frame::Cancel) on
/// the scope before the agent took the message, and the message is
/// gone. A cancel that arrives after the agent took it changes
/// nothing, and the reply is [`Delivered`](Self::Delivered) all the
/// same: the response, not the cancel, says which was first. An
/// [`Error`](Self::Error) is the absence of an answer: no agent of
/// the client's has the name, the agent refused the message's
/// content, or no run could start on it, in the daemon's or the
/// agent's own words.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The agent took the message into its conversation. Tag `0`.
    Delivered,
    /// The client cancelled the message before the agent took it;
    /// the agent never saw it. Tag `1`.
    Cancelled,
    /// A failure. Tag `2`.
    ///
    /// The message was not delivered and will not be. See
    /// [`shared::error::Error`](crate::shared::error::Error)
    /// for why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Delivered`].
const DELIVERED: u8 = 0;

/// Tag for [`Frame::Cancelled`].
const CANCELLED: u8 = 1;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 2;

/// A tag, and — for the error alone — that variant's own JSON.
impl Encode for Frame {
    /// The ordinary JSON failure. The two bare answers cannot fail.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Delivered => {
                out.extend_from_slice(&[DELIVERED]);
                Ok(())
            }
            Frame::Cancelled => {
                out.extend_from_slice(&[CANCELLED]);
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
            DELIVERED => Ok(Frame::Delivered),
            CANCELLED => Ok(Frame::Cancelled),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A message response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's three.
    UnknownTag(u8),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("agents message response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown agents message response frame tag {tag}"),
            FrameError::Error(error) => write!(f, "agents message error did not parse: {error}"),
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
