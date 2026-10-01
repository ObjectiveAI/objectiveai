//! What a client's request frame carries for a create.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{FuseMount, Image, Provider};
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// Ask the daemon to create a tool under a name.
///
/// Everything the tool container is made from — the image, the
/// limits, the mounts, the arguments, and the provider it is pinned
/// to, if any — and the name the tool is held under from then on.
/// Member for member what an agent's template and create name
/// between them, because a tool container is made of what an agent
/// container is made of; what makes it a
/// tool is the image, which runs an MCP server, and what the daemon
/// does with it, which is to serve it to the agents it is attached
/// to. What a caller may not choose is not here at all rather than
/// here and ignored: the container's name, its ports, its entrypoint
/// and its environment are the provider's.
///
/// # The name
///
/// A string of the caller's choosing, unique among the caller's
/// TOOLS — a namespace of its own beside the agents', so a tool and
/// an agent may share a name: the daemon refuses a request whose
/// name is a tool's already, and says so with a variant of its own,
/// because a caller acts on it differently from a failure — use the
/// tool it has, or choose another name. Nothing here constrains the
/// string's form; the daemon compares it and does not read it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    /// The image: a name and a digest. See [`Image`]. It runs an MCP
    /// server, as a tool container's image does.
    pub image: Image,
    /// How much memory the container may have, in BYTES: a ceiling,
    /// as a template's
    /// [`memory`](crate::daemon::endpoints::agents::templates::Template::memory).
    pub memory: u64,
    /// How much the container may WRITE, in BYTES: a ceiling, as a
    /// template's
    /// [`disk`](crate::daemon::endpoints::agents::templates::Template::disk).
    pub disk: u64,
    /// The one provider the tool runs on, and the volumes of that
    /// provider made visible inside the container. See [`Provider`].
    ///
    /// Absent, the tool runs on whichever provider the daemon
    /// chooses, and mounts no volume. A tool pinned to a provider is
    /// served to an agent on any provider: the daemon speaks to the
    /// tool container over the provider it runs on, and answers the
    /// agent's calls on the agent's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<Provider>,
    /// Files of providers' volumes served live across the daemon,
    /// mounted one each over FUSE: see [`FuseMount`], and the agents
    /// create's
    /// [`fuse_file_mounts`](crate::daemon::endpoints::agents::create::client::request::Frame::fuse_file_mounts).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fuse_file_mounts: Vec<FuseMount>,
    /// Directories of providers' volumes served live across the
    /// daemon, mounted one each over FUSE: see [`FuseMount`], and the
    /// agents create's
    /// [`fuse_directory_mounts`](crate::daemon::endpoints::agents::create::client::request::Frame::fuse_directory_mounts).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fuse_directory_mounts: Vec<FuseMount>,
    /// What the image is told once, as the image defines it, for the
    /// tool's life.
    ///
    /// A JSON value, because this crate does not know what an image
    /// takes — a tool server's own knobs — and a wire that typed it
    /// would have to be revised for every image that ever ran. It is
    /// handed to the container and not read here.
    pub arguments: Value,
    /// The name, unique among the caller's tools.
    pub name: String,
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
const TAG: u8 = 9;

/// JSON, as the agents create is: the arguments are a JSON value,
/// and a value cannot come back out of postcard at all.
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

/// A tools create request frame that could not be read.
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
            FrameError::Empty => f.write_str("tools create request frame is empty"),
            FrameError::UnexpectedTag(tag) => {
                write!(f, "expected tools create request tag {TAG}, found {tag}")
            }
            FrameError::Body(error) => {
                write!(f, "tools create request did not parse: {error}")
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
