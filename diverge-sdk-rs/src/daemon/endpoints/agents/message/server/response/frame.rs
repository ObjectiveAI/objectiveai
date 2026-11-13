//! What a server's response frame carries for a message.

use serde::Serialize;
use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::containers::info::Modality;
use crate::shared::error::Error;

/// A message's answer: the agent's program has it, or a failure.
///
/// A message is one question and one reply, so there is one of these
/// per scope, before the finish that ends it — but the reply comes
/// when the fate is known, which may be long after the ask: a message
/// for a sleeping agent waits for its run to be activated and its
/// loop to start, and nothing times it out. A payload leads with one
/// byte saying which — `0` for [`Delivered`](Self::Delivered), `1` for
/// [`Forbidden`](Self::Forbidden), `2` for [`Error`](Self::Error), `3`
/// for [`NoChat`](Self::NoChat), `4` for
/// [`UnsupportedModalities`](Self::UnsupportedModalities) — and only
/// the error and the modalities carry anything after it.
///
/// # What the agent's image says
///
/// The daemon holds the message to the `chat` of the agent's image's
/// `/DIVERGE.md`, as it has it on record, before anything is sent on:
/// [`NoChat`](Self::NoChat) where the image states `false`, and
/// [`UnsupportedModalities`](Self::UnsupportedModalities) where the
/// message carries what its `prompt_modalities` does not take. Either
/// way the message reached no provider and no container, nothing was
/// started for it, and the agent's log has no line of it.
///
/// # A finish with nothing before it
///
/// A request the daemon could not read, as
/// [`endpoints`](crate::daemon::endpoints) states. Nothing else ends
/// the scope without an answer: nothing takes a message back.
///
/// # Error
///
/// An [`Error`](Self::Error) is the absence of a delivery: no agent of
/// the client's is the one named, the agent refused the message's
/// content, or no run could start on it, in the daemon's or the agent's
/// own words.
///
/// # Forbidden
///
/// [`Forbidden`](Self::Forbidden) is an answer every endpoint has: the
/// account the request is served for — the connection's — or the tool
/// the request came from holds no grant allowing
/// what the request asks over what it names, and nothing changed. Where
/// the answer is a stream it comes as an error does: exactly one, with
/// nothing before it, then the finish. Nothing is retried. See
/// [`grant`](crate::daemon::grant).
///
/// # As JSON
///
/// For a client that prints what the daemon answered: the variant,
/// snake case, over its payload — `{"found":…}`, `{"error":…}` — and
/// a bare variant the string alone — `"not_found"`. The wire is the
/// tag byte and the payload, never this.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Frame {
    /// The agent's program has the message: the loop it started took
    /// it, or the program accepted it onto its own queue, taken at its
    /// next seam. Tag `0`.
    Delivered,
    /// The account the request is served for holds no grant allowing
    /// it; nothing changed. Tag `1`.
    Forbidden,
    /// A failure. Tag `2`.
    ///
    /// The message was not delivered and will not be. See
    /// [`shared::error::Error`](crate::shared::error::Error)
    /// for why it says so little.
    Error(Error),
    /// The agent's image does not chat: its document states `chat` as
    /// `false`, or leaves it out. Tag `3`.
    ///
    /// No message is taken, whatever it carries.
    NoChat,
    /// The message carries what the agent's image does not take: the
    /// modalities its content carries and the image's
    /// `chat.prompt_modalities` state as `false`, each once, in the
    /// order image, audio, file, and never empty. Tag `4`.
    ///
    /// The whole message is refused, the text in it included.
    UnsupportedModalities(Vec<Modality>),
}

/// Tag for [`Frame::Delivered`].
const DELIVERED: u8 = 0;

/// Tag for [`Frame::Forbidden`].
const FORBIDDEN: u8 = 1;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 2;

/// Tag for [`Frame::NoChat`].
const NO_CHAT: u8 = 3;

/// Tag for [`Frame::UnsupportedModalities`].
const UNSUPPORTED_MODALITIES: u8 = 4;

/// A tag, and — for the error and the modalities — that variant's own
/// JSON.
impl Encode for Frame {
    /// The ordinary JSON failure. The three bare answers cannot fail.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Delivered => {
                out.extend_from_slice(&[DELIVERED]);
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
            Frame::NoChat => {
                out.extend_from_slice(&[NO_CHAT]);
                Ok(())
            }
            Frame::UnsupportedModalities(modalities) => {
                out.extend_from_slice(&[UNSUPPORTED_MODALITIES]);
                serde_json::to_writer(out, modalities)
            }
        }
    }
}

impl Decode<'_> for Frame {
    /// Four ways to fail, and two of them are JSON.
    type Error = FrameError;

    // Spelled out for the same reason as `encode` above.
    fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            DELIVERED => Ok(Frame::Delivered),
            FORBIDDEN => Ok(Frame::Forbidden),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            NO_CHAT => Ok(Frame::NoChat),
            UNSUPPORTED_MODALITIES => serde_json::from_slice(rest).map(Frame::UnsupportedModalities).map_err(FrameError::Modalities),
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
    /// The modalities did not parse.
    Modalities(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("agents message response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown agents message response frame tag {tag}"),
            FrameError::Error(error) => write!(f, "agents message error did not parse: {error}"),
            FrameError::Modalities(error) => write!(f, "agents message modalities did not parse: {error}"),
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Error(error) | FrameError::Modalities(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
