//! What a client's request frame carries for a create.

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use serde::{Deserialize, Serialize};

use crate::daemon::create::Inner;

/// Ask the daemon to create an agent under a name, from a template.
///
/// What every agent made from the template shares — the image, the
/// limits, the arguments — is the
/// [`template`](Inner::template), named by its id; what is this
/// agent's own is here: the provider it runs on with the volumes it
/// mounts there, its FUSE mounts of providers' volumes, and the name
/// the agent is held under from then on. The dependencies its program
/// declares are deployed by the daemon when its container starts,
/// and nothing of them is named here. The provider is the agent's and not the template's so that a
/// template can be shared. What a caller may not choose is not here at
/// all rather than here and ignored: the container's name, its ports,
/// its entrypoint and its environment are the provider's, because they
/// are how the provider reaches the container and how the container
/// reaches back. There is no environment: the mounts are the caller's
/// only provisioning channel, and a field that is accepted and ignored
/// is a field callers will believe in.
///
/// # The name
///
/// Optional. A string of the caller's choosing, unique among the
/// caller's agents: the daemon refuses a request whose name is an
/// agent's already, and says so with a variant of its own, because a
/// caller acts on it differently from a failure — use the agent it has,
/// or choose another name. A request with no name is never refused for
/// one: the agent is reached by its template and its index, which it
/// always has. Nothing here constrains the string's form; the name is
/// the caller's word for its agent, and the daemon compares it and does
/// not read it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Frame {
    /// What an agent's create and a tool's share — the template, the
    /// provider and its volumes, the FUSE mounts, the account: see
    /// [`Inner`]. Flattened, so its members are this object's own.
    #[serde(flatten)]
    pub inner: Inner,
    /// The name, if any: a string of the caller's choosing, unique
    /// among the caller's agents, by which the agent is reached
    /// afterwards beside its template and its index. Absent, the
    /// agent has none, and is reached once and for all only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
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
const TAG: u8 = 0;

/// JSON, as every request of the daemon's is.
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

/// A agents create request frame that could not be read.
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
            FrameError::Empty => f.write_str("agents create request frame is empty"),
            FrameError::UnexpectedTag(tag) => {
                write!(f, "expected agents create request tag {TAG}, found {tag}")
            }
            FrameError::Body(error) => {
                write!(f, "agents create request did not parse: {error}")
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
