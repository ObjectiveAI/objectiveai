//! What a client's request frame carries for an agentic loop.

use indexmap::IndexMap;
use rmcp::model::ContentBlock;
use serde::{Deserialize, Serialize};

use super::agent::Agent;
use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// What a caller hands a provider to start or resume a loop.
///
/// One shape for both. A resume is this same request with
/// [`continuation`](Self::continuation) set — not a second request
/// type — because everything else still applies: the agent's
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
    /// A prompt, not a conversation. What came before lives in
    /// [`continuation`](Self::continuation), which is the provider's
    /// own state — so a caller sends what is NEW and never
    /// reconstructs a history it would have to keep a parallel record
    /// of.
    ///
    /// Content blocks rather than a message, because the role is
    /// implied: a caller can only ever speak as itself.
    pub prompt: Vec<ContentBlock>,
    /// Resume a loop, using the token from its
    /// [`ContinuationChunk`](crate::endpoints::agentic_loop::run::server::response::ContinuationChunk).
    ///
    /// Opaque: a caller stores it and hands it back, and should read
    /// nothing into its contents.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub continuation: Option<String>,
    /// Files the caller wants present in the container's filesystem,
    /// keyed by absolute container path. Each value is the file's
    /// size-bearing identity:
    /// `f1:<size>:<base64url sha256 of the bytes>`.
    ///
    /// The server MUST mount every entry — read-only — before the
    /// container starts: the request naming an identity IS the
    /// requirement. What the server does not hold it MAY fetch from
    /// the client over the
    /// [`FetchFile`](crate::endpoints::agentic_loop::run::server::channel_request::Frame::FetchFile)
    /// exchange — and because the size rides the identity, it can
    /// refuse an oversized request up front, as a request error,
    /// with nothing fetched.
    ///
    /// Paths are absolute and `/`-separated, with no `.`, `..` or
    /// empty segments; no mount path — file or directory — may be a
    /// prefix of another. Mounting INTO a directory the image owns
    /// is the point; mounts stacking on each other is not.
    ///
    /// An `IndexMap` rather than a `HashMap`: insertion order is
    /// preserved, so the same mount set serializes identically every
    /// time instead of shuffling between runs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_mounts: Option<IndexMap<String, String>>,
    /// Directories likewise, keyed by absolute container path. Each
    /// value is the directory's size-bearing identity:
    /// `d1:<total size>:<base64url sha256 of the manifest>` — the
    /// manifest one `<hash> <size> <path>` line per file, paths
    /// relative and `/`-separated, sorted bytewise.
    ///
    /// Everything [`file_mounts`](Self::file_mounts) says holds here
    /// too, the fetch riding the
    /// [`FetchDirectory`](crate::endpoints::agentic_loop::run::server::channel_request::Frame::FetchDirectory)
    /// exchange instead.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub directory_mounts: Option<IndexMap<String, String>>,
    /// The environment, name to value — set on the container before
    /// it starts, the same shape a laboratory run takes.
    ///
    /// A map rather than a list of `KEY=VALUE` strings, so one name
    /// cannot appear twice with values that contradict each other.
    /// Ordered, so the same environment always serializes identically.
    ///
    /// A provider may reserve names and will win any collision — it
    /// has to, since some of what a container needs is delivered this
    /// way. Which names are reserved is a provider's to state.
    ///
    /// Beside the mounts, this is the caller's other way of
    /// provisioning a run — the channel for TOOL credentials and
    /// harness knobs. Inference auth is NOT provisioned here: where
    /// an agent's provider needs credentials, they are arguments on
    /// the agent itself (hermes's provider structures), so a
    /// provider-reserved name never has to carry them.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub environment: IndexMap<String, String>,
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
