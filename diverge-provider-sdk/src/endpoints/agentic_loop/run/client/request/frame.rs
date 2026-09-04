//! What a client's request frame carries for an agentic loop.

use serde::{Deserialize, Serialize};

use super::Mount;
use super::agent::Agent;
use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// What a caller hands a provider to start or resume a loop.
///
/// One shape for both. A resume is this same request — not a second
/// request type — and the request itself carries nothing that says
/// which: what came before is fetched, not sent. The server opens its
/// [`FetchContinuation`](crate::endpoints::agentic_loop::run::server::channel_request::Frame::FetchContinuation)
/// exchange as the run starts, and the caller answers with the bytes
/// it kept from the last run's close, or with nothing, which is a
/// fresh start. Everything else still applies: the agent's
/// parameters can change between turns, and a resume that could not
/// express that would force a caller to start over to alter them.
///
/// **Everything here is post-transform.** An agent as authored can
/// carry a system prompt, prefix and suffix messages, a personality;
/// those shape a request before a provider sees it, and by the time
/// one of these is built they have already been applied.
/// [`prompt`](Self::prompt) is the result, not the ingredients, so a
/// provider never rewrites what it was given.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    /// What to run, and how to sample it.
    ///
    /// The model and every decoding parameter live here rather than on
    /// the request, because which parameters exist DEPENDS on the
    /// upstream — `logit_bias` is meaningless to Claude Code,
    /// `thinking` is meaningless to OpenRouter, and a Python agent
    /// samples nothing at all.
    pub agent: Agent,
    /// The input for this turn.
    ///
    /// A prompt, not a conversation. What came before lives in the
    /// continuation the server fetches — the provider's own state —
    /// so a caller sends what is NEW and never reconstructs a history
    /// it would have to keep a parallel record of.
    ///
    /// Text rather than a message, because the role is implied — a
    /// caller can only ever speak as itself — and text rather than
    /// content blocks, because every upstream this protocol drives
    /// takes a turn as text; anything richer than text reaches an
    /// agent as a mount, not as the prompt.
    pub prompt: String,
    /// Files the caller wants present in the container's filesystem,
    /// each where it goes and what it is. See [`Mount`].
    ///
    /// What the server does not hold it MAY fetch from the client
    /// over the
    /// [`FetchFile`](crate::endpoints::agentic_loop::run::server::channel_request::Frame::FetchFile)
    /// exchange, by the mount's hash.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub file_mounts: Vec<Mount>,
    /// Directories likewise, the fetch riding the
    /// [`FetchDirectory`](crate::endpoints::agentic_loop::run::server::channel_request::Frame::FetchDirectory)
    /// exchange instead. See [`Mount`].
    ///
    /// The mounts are the caller's ONLY provisioning channel. There
    /// is no environment on this request: every credential is an
    /// argument on the agent — inference auth on its provider, tool
    /// auth on its toolsets — and every other thing a container
    /// needs is a typed field of the agent or a mount. A free map of
    /// names would be a second, untyped way to carry what the typed
    /// fields exist to carry.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub directory_mounts: Vec<Mount>,
}

/// This frame's tag among the scope-opening requests.
///
/// One byte at the front of the payload, which is what tells a reader
/// which request it holds. The frame layer does not discriminate them
/// — [`ClientFrame::Request`](crate::frame::client::ClientFrame::Request)
/// is one type carrying bytes — so the distinction has to be in the
/// bytes, and each request owns the value that names it.
///
/// See the table in [`endpoints`](crate::endpoints) for the whole
/// allocation. The values are chosen across modules that do not know
/// about each other, so the table is the only place they can be seen
/// at once.
const TAG: u8 = 0;

impl Encode for Frame {
    /// The ordinary JSON failure. The tag cannot fail.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        out.extend_from_slice(&[TAG]);
        serde_json::to_writer(out, self)
    }
}

impl Decode<'_> for Frame {
    /// Three ways to fail, and only one of them is JSON.
    type Error = FrameError;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        if *tag != TAG {
            return Err(FrameError::UnexpectedTag(*tag));
        }
        serde_json::from_slice(rest).map_err(FrameError::Body)
    }
}

/// A an agentic loop request request frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag naming some other request.
    ///
    /// A reader that dispatched on the tag will not see this. One that
    /// assumed which request it held, and was wrong, will — which is
    /// the point of checking a tag rather than skipping it.
    UnexpectedTag(u8),
    /// The request did not parse.
    Body(serde_json::Error),
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("an agentic loop request request frame is empty")
            }
            FrameError::UnexpectedTag(tag) => {
                write!(f, "expected an agentic loop request request tag {TAG}, found {tag}")
            }
            FrameError::Body(error) => {
                write!(f, "an agentic loop request request did not parse: {error}")
            }
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Body(error) => Some(error),
            FrameError::Empty | FrameError::UnexpectedTag(_) => None,
        }
    }
}
