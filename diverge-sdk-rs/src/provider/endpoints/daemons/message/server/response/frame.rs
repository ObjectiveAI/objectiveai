//! What a server's response frame carries for a message.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// A message's one answer.
///
/// There is exactly one of these per scope, before the finish that
/// ends it, or none at all — but the answer comes when the fate is
/// known, which may be long after the ask: a message waits in its
/// receiver's queue until the receiver has taken the ones before it,
/// and nothing times it out. A payload leads with one byte saying
/// which — `0` for [`Delivered`](Self::Delivered), `1` for
/// [`NotListening`](Self::NotListening), `2` for
/// [`RateLimited`](Self::RateLimited), `3` for
/// [`Unsupported`](Self::Unsupported), `4` for
/// [`AlreadyOpen`](Self::AlreadyOpen) — and only `RateLimited` carries
/// anything after it. The rules:
///
/// 1. **Received is taken.** `Delivered` is sent once the receiver's
///    reader has taken the message off its queue — the receiver's
///    listen scope's room back, see [`credit`](crate::wire::credit).
/// 2. **Nothing kept.** A receiver with no listen scope open is
///    `NotListening` at once, and so is one whose listen scope ends
///    before it took the message. The provider stores no message for
///    a receiver.
/// 3. **The cancel.** A cancel in time — while the message waits in
///    the receiver's queue — takes it out, and the scope finishes with
///    no response before the finish. A cancel after the provider sent
///    the message to the receiver changes nothing.
/// 4. **The rate.** The rate limit applies to every send, whoever
///    sends it. A send over its identity's window is `RateLimited`,
///    and nothing is queued. What counts toward the window: every send
///    answered `Delivered` or `NotListening`, and every send cancelled
///    in time. What does not: `RateLimited`, `Unsupported` and
///    `AlreadyOpen`. A window begins at the first send it counts, and
///    lasts the provider's `seconds`.
/// 5. **The refusals.** `NotListening`, `RateLimited`, `Unsupported`
///    and `AlreadyOpen` are each sent before anything is queued, when
///    their condition holds at the opening. `Unsupported` is a
///    provider that relays no messages; `AlreadyOpen` is a connection
///    with a message open already — see
///    [the endpoints](crate::provider::endpoints#one-open-at-a-time).
/// 6. **No error.** A provider always knows who is listening, so no
///    response fails.
/// 7. **Any connected identity.** Every connected identity may send,
///    whether or not it holds a
///    [`daemons::announce`](crate::provider::endpoints::daemons::announce)
///    open and whether or not it listens itself: sending requires
///    neither being listed nor listening. The receiver need not be
///    listed either; it need only be listening (rule 2).
///
/// # A finish with nothing is an answer
///
/// The caller cancelled in time, or the request was not served: a
/// scope that finishes with no response before it is that, not a
/// failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Frame {
    /// The receiver's reader took the message (rule 1). Tag `0`.
    Delivered,
    /// The receiver is not listening, or stopped before it took the
    /// message (rule 2). Tag `1`.
    NotListening,
    /// The sender is over its window, and nothing was queued (rule 4).
    /// Tag `2`.
    ///
    /// Carries `retry_after`, a `u32` as a postcard varint: the seconds
    /// until the window ends, rounded up.
    RateLimited {
        /// The seconds until the window ends, rounded up.
        retry_after: u32,
    },
    /// The provider relays no messages: no `messages` object is among
    /// its capabilities, see [`info`](crate::provider::endpoints::info).
    /// Tag `3`.
    Unsupported,
    /// Refused: this connection has a message open already. Tag `4`.
    AlreadyOpen,
}

/// Tag for [`Frame::Delivered`].
const DELIVERED: u8 = 0;

/// Tag for [`Frame::NotListening`].
const NOT_LISTENING: u8 = 1;

/// Tag for [`Frame::RateLimited`].
const RATE_LIMITED: u8 = 2;

/// Tag for [`Frame::Unsupported`].
const UNSUPPORTED: u8 = 3;

/// Tag for [`Frame::AlreadyOpen`].
const ALREADY_OPEN: u8 = 4;

/// The tag, and `RateLimited`'s varint after it.
impl Encode for Frame {
    /// Nothing can fail: a tag, and a varint.
    type Error = std::convert::Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        match self {
            Frame::Delivered => out.extend_from_slice(&[DELIVERED]),
            Frame::NotListening => out.extend_from_slice(&[NOT_LISTENING]),
            Frame::RateLimited { retry_after } => {
                out.extend_from_slice(&[RATE_LIMITED]);
                let mut varint = [0u8; 5];
                // A `u32` is at most five bytes of varint, so the
                // buffer always holds it.
                if let Ok(written) = postcard::to_slice(retry_after, &mut varint) {
                    out.extend_from_slice(written);
                }
            }
            Frame::Unsupported => out.extend_from_slice(&[UNSUPPORTED]),
            Frame::AlreadyOpen => out.extend_from_slice(&[ALREADY_OPEN]),
        }
        Ok(())
    }
}

impl Decode<'_> for Frame {
    /// Three ways to fail.
    type Error = FrameError;

    fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            DELIVERED => Ok(Frame::Delivered),
            NOT_LISTENING => Ok(Frame::NotListening),
            RATE_LIMITED => postcard::from_bytes(rest).map(|retry_after| Frame::RateLimited { retry_after }).map_err(FrameError::RetryAfter),
            UNSUPPORTED => Ok(Frame::Unsupported),
            ALREADY_OPEN => Ok(Frame::AlreadyOpen),
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
    /// `RateLimited`'s seconds did not parse.
    RetryAfter(postcard::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("message response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown message response frame tag {tag}"),
            FrameError::RetryAfter(error) => write!(f, "message retry_after did not parse: {error}"),
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::RetryAfter(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
