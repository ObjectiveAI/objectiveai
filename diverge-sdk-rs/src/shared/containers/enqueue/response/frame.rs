//! The answer to an enqueue: the batch's fate.

use std::error;
use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared;

/// One frame, then the finish: what became of the enqueued batch,
/// every message in it alike.
///
/// A payload leads with one byte saying which; only
/// [`Error`](Self::Error) carries anything after it. The fate is
/// data-free deliberately — the fate IS the answer, and the
/// messages' content is the client's own to remember.
///
/// # The channel stays open until there is a fate
///
/// An enqueued batch can wait for a loop to start — the run being
/// activated, or the one in flight ending — so this answer can
/// arrive long after the ask. Nothing times it out — nothing in this
/// protocol times anything out — and every enqueued batch gets
/// exactly one of these eventually: handed to the agent's program,
/// by the run it starts or onto the queue of the one in flight; or
/// the error, when the agent refused it or no run could start on it.
/// A run ending does not lose a batch: what it left waiting starts
/// the next.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The agent's program has every message of the batch. Tag `0`.
    ///
    /// The loop it started took it, or the loop in flight's program
    /// accepted it onto its own queue, to take at its next seam —
    /// folded in beside tool results, or opening the next turn. WHERE
    /// it landed is visible in the scope's main stream; this says
    /// only that it is the program's. Tag `1` is retired.
    Delivered,
    /// The agent refused the batch, or no run could start on it.
    /// Tag `2`.
    ///
    /// The one way a batch is lost, in the agent's server's own
    /// words, in the protocol's one error shape: it would not take
    /// the batch's content — at the start of a run, or at the
    /// delivery of a queued batch into the loop in flight — or it
    /// could not be reached when the batch was to start a run. A
    /// loop that merely ends with the batch waiting does not give
    /// this; the batch starts the next.
    Error(shared::error::Error),
}

/// Tag for [`Frame::Delivered`].
const DELIVERED: u8 = 0;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 2;

/// A tag, and — for the error alone — that variant's own JSON.
impl Encode for Frame {
    /// The ordinary JSON failure. `Delivered` cannot fail at all.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Delivered => {
                out.extend_from_slice(&[DELIVERED]);
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
    /// A tag that is none of this frame's two.
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
