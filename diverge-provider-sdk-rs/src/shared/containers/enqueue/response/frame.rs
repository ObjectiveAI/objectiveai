//! The answer to an enqueue: the message's fate.

use std::error;
use std::fmt;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::shared;

/// One frame, then the finish: what became of the enqueued message.
///
/// A payload leads with one byte saying which; only
/// [`Error`](Self::Error) carries anything after it. The three fates
/// are data-free deliberately — the fate IS the answer, and the
/// message's content is the client's own to remember.
///
/// # The channel stays open until there is a fate
///
/// An enqueued message can sit in the queue for as long as the agent
/// takes to reach a seam, so this answer can arrive long after the
/// ask. Nothing times it out — nothing in this protocol times
/// anything out — and every enqueued message gets exactly one of
/// these eventually, because every queue ends: taken, withdrawn, or
/// outlived.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The agent took the message into the conversation. Tag `0`.
    ///
    /// Folded in beside tool results mid-loop, opening the next
    /// turn, or starting a loop when none ran — WHERE it landed is
    /// visible in the scope's main stream; this says only that it
    /// did.
    Delivered,
    /// The caller withdrew the message before the agent took it.
    /// Tag `1`.
    Dequeued,
    /// The run ended before the message could be taken. Tag `2`.
    ///
    /// Nothing malfunctioned and nobody withdrew it — there was no
    /// conversation left for it to enter.
    Missed,
    /// The message's fate could not be determined. Tag `3`.
    ///
    /// The provider's own failure, in the protocol's one error shape
    /// — not a fate of the message but the absence of one.
    Error(shared::error::Error),
}

/// Tag for [`Frame::Delivered`].
const DELIVERED: u8 = 0;

/// Tag for [`Frame::Dequeued`].
const DEQUEUED: u8 = 1;

/// Tag for [`Frame::Missed`].
const MISSED: u8 = 2;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 3;

/// A tag, and — for the error alone — that variant's own JSON.
impl Encode for Frame {
    /// The ordinary JSON failure. Three variants cannot fail at all.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Delivered => {
                out.extend_from_slice(&[DELIVERED]);
                Ok(())
            }
            Frame::Dequeued => {
                out.extend_from_slice(&[DEQUEUED]);
                Ok(())
            }
            Frame::Missed => {
                out.extend_from_slice(&[MISSED]);
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
            DEQUEUED => Ok(Frame::Dequeued),
            MISSED => Ok(Frame::Missed),
            ERROR => shared::error::Error::decode(rest)
                .map(Frame::Error)
                .map_err(FrameError::Body),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// An enqueue answer that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's four.
    ///
    /// What a provider newer than its caller produces, which is the
    /// case the tag exists to make survivable: a reader that does not
    /// know a variant says so, rather than reading somebody else's
    /// bytes as its own.
    UnknownTag(u8),
    /// The error payload after the tag did not parse.
    Body(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("enqueue answer frame is empty"),
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown enqueue answer tag {tag}")
            }
            FrameError::Body(error) => {
                write!(f, "enqueue answer error did not parse: {error}")
            }
        }
    }
}

impl error::Error for FrameError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            FrameError::Body(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
