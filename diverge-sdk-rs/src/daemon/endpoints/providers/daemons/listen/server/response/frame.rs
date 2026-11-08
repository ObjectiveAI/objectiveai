//! What a server's response frame carries for a listen.

use serde::Serialize;
use std::fmt;

use bytes::Bytes;

use crate::shared::error::Error;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

use super::Message;
use super::message::Head;

/// A listen's answer: a message, the count of those missed, forbidden,
/// or a failure.
///
/// A listen is a stream kept open. Zero or more
/// [`Message`](Self::Message)s and [`Skipped`](Self::Skipped)s, for the
/// scope's life; the finish follows the client's cancel, the client's
/// connection ending, or exactly one [`Error`](Self::Error); or the
/// scope is exactly one [`Forbidden`](Self::Forbidden), then the
/// finish. A payload leads with one byte saying which — `0` for
/// `Message`, `1` for `Skipped`, `2` for `Forbidden`, `3` for `Error`.
/// The rules:
///
/// 1. **The message on the wire.** After the tag, a `u32`, big-endian,
///    then that many bytes of JSON — an object with the members
///    `provider` and `from`, each an identity — then the body, raw to
///    the end of the payload, as a download's chunk lays out its path
///    and its body.
/// 2. **In the order taken.** Messages come in the order the daemon
///    took them from its providers, whichever provider each came
///    through.
/// 3. **Skipped.** A listener that fell further behind than the daemon
///    keeps is sent exactly one `Skipped`, carrying how many messages
///    it missed, as JSON, and the messages after them follow.
///
/// # A finish with nothing is an answer
///
/// The client cancelled before anything was sent, or the request was
/// not served: a scope that finishes with no response before it is
/// that, not a failure. An [`Error`](Self::Error) is a failure: the
/// daemon could not listen, for whatever reason it knows.
///
/// # Forbidden
///
/// [`Forbidden`](Self::Forbidden) is an answer every endpoint has: the
/// account the request is served for — the connection's — or the tool
/// the request came from holds no grant allowing what the request asks,
/// and nothing changed. It comes as an error does: exactly one, with
/// nothing before it, then the finish. See
/// [`grant`](crate::daemon::grant).
///
/// # As JSON
///
/// For a client that prints what the daemon answered: the variant,
/// snake case, over its payload — `{"message":…}`, `{"skipped":…}` —
/// and a bare variant the string alone — `"forbidden"`. The body is
/// standard base64. The wire is the tag byte and the payload, never
/// this.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Frame {
    /// One message sent to the daemon (rule 1). Tag `0`.
    Message(Message),
    /// The listener fell further behind than the daemon keeps, and
    /// missed `count` messages (rule 3). Tag `1`.
    Skipped {
        /// How many messages it missed.
        count: u64,
    },
    /// The account the request is served for holds no grant allowing
    /// it; nothing was sent. Tag `2`.
    Forbidden,
    /// A failure. Tag `3`.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Message`].
const MESSAGE: u8 = 0;

/// Tag for [`Frame::Skipped`].
const SKIPPED: u8 = 1;

/// Tag for [`Frame::Forbidden`].
const FORBIDDEN: u8 = 2;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 3;

/// The `Skipped` payload's JSON.
#[derive(Serialize, serde::Deserialize)]
struct SkippedJson {
    count: u64,
}

impl Frame {
    /// Write a message's payload from its parts, as [`Frame::Message`]
    /// would be written: what a daemon that holds the body apart from
    /// any [`Message`] writes, with no copy of the body before the one
    /// into `out`.
    pub fn encode_message(provider: &crate::shared::Identity, from: &crate::shared::Identity, body: &[u8], out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        let head = serde_json::to_vec(&Head { provider: provider.clone(), from: from.clone() })?;
        let length = u32::try_from(head.len()).unwrap_or(u32::MAX);
        out.extend_from_slice(&[MESSAGE]);
        out.extend_from_slice(&length.to_be_bytes());
        out.extend_from_slice(&head);
        out.extend_from_slice(body);
        Ok(())
    }
}

/// A tag, then the variant's own payload, if it has any.
impl Encode for Frame {
    /// The ordinary JSON failure. The bare answers cannot fail.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Message(message) => Frame::encode_message(&message.provider, &message.from, &message.body, out),
            Frame::Skipped { count } => {
                out.extend_from_slice(&[SKIPPED]);
                serde_json::to_writer(out, &SkippedJson { count: *count })
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
    /// Four ways to fail, and each names which part failed.
    type Error = FrameError;

    // Spelled out for the same reason as `encode` above.
    fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            MESSAGE => {
                let (length, rest) = rest.split_first_chunk::<4>().ok_or(FrameError::Short)?;
                let length = usize::try_from(u32::from_be_bytes(*length)).map_err(|_| FrameError::Short)?;
                if rest.len() < length {
                    return Err(FrameError::Short);
                }
                let (head, body) = rest.split_at(length);
                let head: Head = serde_json::from_slice(head).map_err(FrameError::Message)?;
                Ok(Frame::Message(Message { provider: head.provider, from: head.from, body: Bytes::copy_from_slice(body) }))
            }
            SKIPPED => serde_json::from_slice::<SkippedJson>(rest).map(|skipped| Frame::Skipped { count: skipped.count }).map_err(FrameError::Skipped),
            FORBIDDEN => Ok(Frame::Forbidden),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A listen response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's four.
    UnknownTag(u8),
    /// A message shorter than its length says.
    Short,
    /// A message's identities did not parse.
    Message(serde_json::Error),
    /// A count of messages missed did not parse.
    Skipped(serde_json::Error),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("providers daemons listen response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown providers daemons listen response frame tag {tag}"),
            FrameError::Short => f.write_str("providers daemons listen message is shorter than its length"),
            FrameError::Message(error) => write!(f, "providers daemons listen message did not parse: {error}"),
            FrameError::Skipped(error) => write!(f, "providers daemons listen skipped count did not parse: {error}"),
            FrameError::Error(error) => write!(f, "providers daemons listen error did not parse: {error}"),
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Message(error) | FrameError::Skipped(error) | FrameError::Error(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) | FrameError::Short => None,
        }
    }
}
