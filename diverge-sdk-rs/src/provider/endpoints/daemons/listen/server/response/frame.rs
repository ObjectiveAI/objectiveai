//! What a server's response frame carries for a listen.

use std::fmt;

use bytes::Bytes;

use super::Message;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// A message sent to the caller, or the refusal of the listen.
///
/// A listen is a stream kept open. A payload leads with one byte
/// saying which — `0` for [`Message`](Self::Message), `1` for
/// [`Unsupported`](Self::Unsupported), `2` for
/// [`AlreadyOpen`](Self::AlreadyOpen) — and the rest is that variant's
/// own bytes. The rules:
///
/// 1. **The messages.** Zero or more `Message`s, one per message sent
///    to the caller's identity while the scope is open, in the order
///    the provider queued them.
/// 2. **One ahead.** The provider sends a message only once the caller
///    has taken the one before it — the scope's room back, see
///    [`credit`](crate::wire::credit) — and answers that message's
///    sender delivered then.
/// 3. **No end of its own.** The finish follows the caller's cancel or
///    the caller's connection ending, and nothing else.
/// 4. **The refusals.** A provider that relays no messages sends
///    `Unsupported` first and alone, and the finish follows it. A
///    connection with a listen open already, or an identity with a
///    listen open on another connection, is sent `AlreadyOpen` first
///    and alone, and the finish follows it — see
///    [the endpoints](crate::provider::endpoints#one-open-at-a-time).
/// 5. **No error.** A provider always knows who is listening, so no
///    response fails.
/// 6. **Any connected identity.** Every connected identity may listen,
///    whether or not it holds a
///    [`daemons::announce`](crate::provider::endpoints::daemons::announce)
///    open: listening does not require being listed.
///
/// # A finish with nothing is an answer
///
/// The caller cancelled before anything was sent, or the request was
/// not served: a scope that finishes with no response before it is
/// that, not a failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Frame {
    /// A message sent to the caller. Tag `0`.
    ///
    /// The sender's identity as postcard — its `issuer`, then its
    /// `subject`, each a varint byte length and that many bytes of
    /// UTF-8 — and then the body, running to the end of the payload.
    /// There is no length before the body, because the payload already
    /// ends where the body does.
    Message(Message),
    /// The provider relays no messages: no `messages` object is among
    /// its capabilities, see [`info`](crate::provider::endpoints::info).
    /// Tag `1`.
    ///
    /// Carries nothing. Sent first and alone (rule 4).
    Unsupported,
    /// Refused: a listen is open on this connection already, or under
    /// the caller's identity on another. Tag `2`.
    ///
    /// Carries nothing. Sent first and alone (rule 4).
    AlreadyOpen,
}

/// Tag for [`Frame::Message`].
const MESSAGE: u8 = 0;

/// Tag for [`Frame::Unsupported`].
const UNSUPPORTED: u8 = 1;

/// Tag for [`Frame::AlreadyOpen`].
const ALREADY_OPEN: u8 = 2;

impl Frame {
    /// Write a message's payload from its parts, as
    /// [`Frame::Message`] would be written: what a provider that holds
    /// the body apart from any [`Message`] writes, with no copy of the
    /// body before the one into `out`.
    pub fn encode_message(from: &crate::shared::Identity, body: &[u8], out: &mut Writer<'_>) -> Result<(), FrameEncodeError> {
        out.extend_from_slice(&[MESSAGE]);
        postcard::to_io(from, &mut *out).map(|_| ()).map_err(FrameEncodeError::Identity)?;
        out.extend_from_slice(body);
        Ok(())
    }
}

/// The tag, then the sender's identity as postcard and the body as it
/// is; the refusals are their tag alone.
impl Encode for Frame {
    /// The identity did not serialize.
    type Error = FrameEncodeError;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), FrameEncodeError> {
        match self {
            Frame::Message(message) => Frame::encode_message(&message.from, &message.body, out),
            Frame::Unsupported => {
                out.extend_from_slice(&[UNSUPPORTED]);
                Ok(())
            }
            Frame::AlreadyOpen => {
                out.extend_from_slice(&[ALREADY_OPEN]);
                Ok(())
            }
        }
    }
}

impl Decode<'_> for Frame {
    /// Three ways to fail.
    type Error = FrameDecodeError;

    fn decode(bytes: &[u8]) -> Result<Self, FrameDecodeError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameDecodeError::Empty)?;
        match *tag {
            MESSAGE => {
                let (from, body) = postcard::take_from_bytes(rest).map_err(FrameDecodeError::Identity)?;
                Ok(Frame::Message(Message { from, body: Bytes::copy_from_slice(body) }))
            }
            UNSUPPORTED => Ok(Frame::Unsupported),
            ALREADY_OPEN => Ok(Frame::AlreadyOpen),
            tag => Err(FrameDecodeError::UnknownTag(tag)),
        }
    }
}

/// A listen response that could not be written.
#[derive(Debug)]
pub enum FrameEncodeError {
    /// The identity did not serialize.
    Identity(postcard::Error),
}

impl fmt::Display for FrameEncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameEncodeError::Identity(error) => write!(f, "listen message identity did not serialize: {error}"),
        }
    }
}

impl std::error::Error for FrameEncodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameEncodeError::Identity(error) => Some(error),
        }
    }
}

/// A listen response that could not be read.
#[derive(Debug)]
pub enum FrameDecodeError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's three.
    UnknownTag(u8),
    /// The sender's identity did not parse.
    Identity(postcard::Error),
}

impl fmt::Display for FrameDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameDecodeError::Empty => f.write_str("listen response frame is empty"),
            FrameDecodeError::UnknownTag(tag) => write!(f, "unknown listen response frame tag {tag}"),
            FrameDecodeError::Identity(error) => write!(f, "listen message identity did not parse: {error}"),
        }
    }
}

impl std::error::Error for FrameDecodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameDecodeError::Identity(error) => Some(error),
            FrameDecodeError::Empty | FrameDecodeError::UnknownTag(_) => None,
        }
    }
}
