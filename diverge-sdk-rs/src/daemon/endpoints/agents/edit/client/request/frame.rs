//! What a client's request frame carries for an edit.

use serde::{Deserialize, Serialize};

use crate::daemon::patch::Patch;
use crate::daemon::reference;
use crate::shared::relay::Relay;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// Ask the daemon to change an agent: a patch to its editable
/// members — its name and its mounts.
///
/// The agent is named by its name or by its template and its index, as
/// [`reference`](crate::daemon::reference) states. The patch is
/// applied to the agent's [`Editable`](super::Editable) document, whole or not at all,
/// and the result is the agent from then on. An agent whose container
/// is running takes one kind of change: FUSE mounts appended to
/// either list, each mounted into the running container before the
/// edit persists — the edit answers once the provider has mounted
/// them, and what the provider mounted is what persists; a result
/// differing in anything else is answered `Active`, and nothing
/// changed. The agent's image, limits, provider and arguments are not
/// edited: they are its for its life; which vault key each of its
/// secrets is, is.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Frame {
    /// The agent: by its name, as its create gave it, or by its
    /// template and its index, which name it once and for all. See
    /// [`reference::Agent`].
    pub agent: reference::Agent,
    /// The patch: see [`Patch`], and [`Editable`](super::Editable) for the document it
    /// is applied to. Empty, nothing changes.
    pub patch: Patch,
    /// Served by another daemon, as one of the accounts this daemon
    /// holds on it: see [`Relay`]. Absent, served here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relay: Option<Relay>,
}
/// This frame's tag among the scope-opening requests.
///
/// One byte at the front of the payload, which is what tells a reader
/// which request it holds. The frame layer does not discriminate them —
/// [`ClientFrame::Request`](crate::wire::frame::client::ClientFrame::Request)
/// is one type carrying bytes — so the distinction has to be in the
/// bytes, and each request owns the value that names it.
///
/// See the table in [`endpoints`](crate::daemon::endpoints) for the
/// whole allocation. The values are chosen across modules that do not
/// know about each other, so the table is the only place they can be
/// seen at once.
const TAG: u8 = 6;

/// JSON, as the create is: the same types, and the same reader for
/// every request of the agents family.
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

/// An agents edit request frame that could not be read.
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
            FrameError::Empty => f.write_str("agents edit request frame is empty"),
            FrameError::UnexpectedTag(tag) => {
                write!(f, "expected agents edit request tag {TAG}, found {tag}")
            }
            FrameError::Body(error) => {
                write!(f, "agents edit request did not parse: {error}")
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
