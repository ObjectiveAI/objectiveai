//! What a server's response frame carries for a message.

use serde::Serialize;
use std::fmt;

use crate::shared::error::Error;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// A message's answer.
///
/// Exactly one of these per scope, before the finish that ends it, or
/// none at all: a cancel in time — before the message was sent through
/// a provider, or while it waited there — ends the scope with the
/// finish and no response before it, and a cancel after the receiver
/// had it changes nothing. The answer comes when the fate is known,
/// which may be long after the ask: a message waits for a provider with
/// room, and then for the receiver's reader. A payload leads with one
/// byte saying which — `0` for [`Delivered`](Self::Delivered), `1` for
/// [`NotListening`](Self::NotListening), `2` for
/// [`NotFound`](Self::NotFound), `3` for [`Forbidden`](Self::Forbidden),
/// `4` for [`Error`](Self::Error) — and only the error carries anything
/// after it.
///
/// # Forbidden
///
/// [`Forbidden`](Self::Forbidden) is an answer every endpoint has: the
/// account the request is served for holds no grant allowing what the
/// request asks over what it names, and nothing was sent — here, no
/// providers grant whose `message` reaches a provider where the
/// identity is listed. See [`grant`](crate::daemon::grant).
///
/// # As JSON
///
/// For a client that prints what the daemon answered: the variant,
/// snake case — `"delivered"`, `{"error":…}`. The wire is the tag byte
/// and the payload, never this.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Frame {
    /// The receiver's reader took the message. Tag `0`.
    Delivered,
    /// The identity is listed through a provider the account may send
    /// through, but listens through none, or stopped before it took the
    /// message. Tag `1`.
    NotListening,
    /// No connected provider's daemons list holds the identity.
    /// Tag `2`.
    NotFound,
    /// The account holds no grant allowing it, or the identity is
    /// listed only through providers the grant does not reach; nothing
    /// was sent. Tag `3`.
    Forbidden,
    /// A failure. Tag `4`.
    ///
    /// The message was not delivered and will not be. See
    /// [`shared::error::Error`](crate::shared::error::Error) for why it
    /// says so little.
    Error(Error),
}

/// Tag for [`Frame::Delivered`].
const DELIVERED: u8 = 0;

/// Tag for [`Frame::NotListening`].
const NOT_LISTENING: u8 = 1;

/// Tag for [`Frame::NotFound`].
const NOT_FOUND: u8 = 2;

/// Tag for [`Frame::Forbidden`].
const FORBIDDEN: u8 = 3;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 4;

/// A tag, then the error's JSON for an error, nothing otherwise.
impl Encode for Frame {
    /// The ordinary JSON failure. The bare answers cannot fail.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Delivered => {
                out.extend_from_slice(&[DELIVERED]);
                Ok(())
            }
            Frame::NotListening => {
                out.extend_from_slice(&[NOT_LISTENING]);
                Ok(())
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
    /// Three ways to fail.
    type Error = FrameError;

    // Spelled out for the same reason as `encode` above.
    fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            DELIVERED => Ok(Frame::Delivered),
            NOT_LISTENING => Ok(Frame::NotListening),
            NOT_FOUND => Ok(Frame::NotFound),
            FORBIDDEN => Ok(Frame::Forbidden),
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
    /// A tag that is none of this frame's five.
    UnknownTag(u8),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("providers daemons message response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown providers daemons message response frame tag {tag}"),
            FrameError::Error(error) => write!(f, "providers daemons message error did not parse: {error}"),
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
