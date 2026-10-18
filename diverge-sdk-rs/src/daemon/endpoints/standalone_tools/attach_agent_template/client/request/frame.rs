//! What a client's request frame carries for an attach.

use serde::{Deserialize, Serialize};
use crate::shared::relay::Relay;

use crate::daemon::reference;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// Ask the daemon to attach a tool to every agent of a template.
///
/// The tool's name is the one its
/// [`create`](crate::daemon::endpoints::standalone_tools::create) gave
/// it; the template is named by its id. Attaching is idempotent: a
/// tool already attached to the template is attached to it after, and
/// the daemon says so as it says it of a first attach. Every agent of
/// the template — made already or made later, active or idle — has
/// the tool from now on: a running one's proxy serves it from this
/// instant and the agent is told its tool list changed.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Frame {
    /// The tool: by its name, as its create or its connect gave it,
    /// or by its template and its index, which name a created tool
    /// once and for all. See [`reference::Tool`].
    pub tool: reference::Tool,
    /// The agent template, by its id: the hash every agent made from
    /// it carries as its `template`. See
    /// [`templates`](crate::daemon::endpoints::agents::templates).
    pub template: String,
    /// Served by another daemon, as one of the accounts this daemon
    /// holds on it: see [`Relay`]. Absent, served here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relay: Option<Relay>,
}

/// This frame's tag among the scope-opening requests.
///
/// One byte at the front of the payload, which is what tells a reader
/// which request it holds. The frame layer does not discriminate them
/// — [`ClientFrame::Request`](crate::wire::frame::client::ClientFrame::Request)
/// is one type carrying bytes — so the distinction has to be in the
/// bytes, and each request owns the value that names it.
///
/// See the table in [`endpoints`](crate::daemon::endpoints) for the whole
/// allocation. The values are chosen across modules that do not know
/// about each other, so the table is the only place they can be seen
/// at once.
const TAG: u8 = 101;

/// JSON, as the rest of the daemon's requests are: two strings.
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

/// A standalone tools attach agent template request frame that could not be read.
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
            FrameError::Empty => f.write_str("standalone tools attach agent template request frame is empty"),
            FrameError::UnexpectedTag(tag) => {
                write!(f, "expected standalone tools attach agent template request tag {TAG}, found {tag}")
            }
            FrameError::Body(error) => {
                write!(f, "standalone tools attach agent template request did not parse: {error}")
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
