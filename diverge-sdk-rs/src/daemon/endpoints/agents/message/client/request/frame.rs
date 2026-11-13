//! What a client's request frame carries for a message.

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use serde::{Deserialize, Serialize};
use crate::shared::relay::Relay;

use crate::daemon::reference;

use super::Message;

/// Ask the daemon to ask an agent something.
///
/// The name is the one a [`create`](crate::daemon::endpoints::agents::create)
/// gave the agent. What is asked is the frame's `type` and the members
/// of that kind, beside its own: see [`Message`]. What the agent makes
/// of a block it is sent is its image's; an image that cannot take one
/// refuses the whole request, and the scope's error carries its words.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Frame {
    /// The agent: by its name, as its create gave it, or by its
    /// template and its index, which name it once and for all. See
    /// [`reference::Agent`].
    pub agent: reference::Agent,
    /// What is asked: its `type`, and that kind's members.
    #[serde(flatten)]
    pub message: Message,
    /// Whether to watch: `true` has the answer be the part of the
    /// agent's log that is the request's, as the
    /// [response](super::super::server::response::Frame) states.
    /// Absent or `false`, the answer is the request's fate alone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub watch: Option<bool>,
    /// Served by another daemon: see [`Relay`]. Absent, served here.
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
const TAG: u8 = 3;

/// JSON, as the enqueue is on the provider's wire: the content is
/// MCP's, and MCP is JSON.
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

/// A message request frame that could not be read.
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
            FrameError::Empty => f.write_str("agents message request frame is empty"),
            FrameError::UnexpectedTag(tag) => {
                write!(f, "expected agents message request tag {TAG}, found {tag}")
            }
            FrameError::Body(error) => {
                write!(f, "agents message request did not parse: {error}")
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
