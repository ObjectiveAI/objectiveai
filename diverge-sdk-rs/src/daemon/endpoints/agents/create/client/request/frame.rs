//! What a client's request frame carries for a create.

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use serde::{Deserialize, Serialize};

use crate::daemon::daemon_tools::DaemonTools;
use super::{FuseMount, Provider};

/// Ask the daemon to create an agent under a name, from a template.
///
/// What every agent made from the template shares — the image, the
/// limits, the resources, the arguments — is the
/// [`template`](Self::template), named by its id; what is this
/// agent's own is here: the provider it runs on with the volumes it
/// mounts there, its FUSE mounts of providers' volumes, the daemon's
/// own tools it holds, and the name the agent is held under from
/// then on. The provider is the agent's
/// and not the template's so that a template can be shared. What a caller may not choose is not here at
/// all rather than here and ignored: the container's name, its
/// ports, its entrypoint and its environment are the provider's,
/// because they are how the provider reaches the container and how
/// the container reaches back. There is no environment: the mounts
/// are the caller's only provisioning channel, and a field that is
/// accepted and ignored is a field callers will believe in.
///
/// # The name
///
/// A string of the caller's choosing, unique among the caller's
/// agents: the daemon refuses a request whose name is an agent's
/// already, and says so with a variant of its own, because a caller
/// acts on it differently from a failure — use the agent it has, or
/// choose another name. Nothing here constrains the string's form;
/// the name is the caller's word for its agent, and the daemon
/// compares it and does not read it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Frame {
    /// The template the agent is made from, by its id — the hash a
    /// [`templates::create`](crate::daemon::endpoints::agents::templates::create)
    /// answered. An id no template of the caller's has is the
    /// create's error. The template's image, limits, resources and
    /// arguments are the agent's for its life.
    pub template: String,
    /// The one provider the agent runs on, and the volumes of that
    /// provider made visible inside the container. See [`Provider`].
    ///
    /// Absent, the agent runs on whichever provider the daemon
    /// chooses, and mounts no volume: a volume is a provider's own
    /// and does not carry across, so an agent with state on a
    /// provider's disk is an agent of that provider. The agent's for
    /// its life, and never the template's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<Provider>,
    /// Files of providers' volumes served LIVE across the daemon,
    /// mounted one each over FUSE.
    ///
    /// Each names a file by a provider, a volume of that provider's
    /// and a path in it, and its path in the container — see
    /// [`FuseMount`]; each may name a different provider. Every one
    /// is mounted before the agent runs, and every read and every
    /// write of a piece inside the container is one ask, forwarded by
    /// the daemon to the volume's provider and served from the
    /// volume's file in place. The file is
    /// overwritten in place only; a program that replaces its file by
    /// rename needs a directory mount. Its container path is no other
    /// mount's — the template's resource mounts included — and lies
    /// inside none, as every mount's.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fuse_file_mounts: Vec<FuseMount>,
    /// Directories of providers' volumes served LIVE across the
    /// daemon, mounted one each over FUSE.
    ///
    /// Each names a directory by a provider, a volume of that
    /// provider's and a path in it, and its path in the container —
    /// see [`FuseMount`]; each may name a different provider. The
    /// whole tree under the volume path is what the container sees:
    /// every listing, stat, read or write of a piece, truncation,
    /// change of attributes, creation, removal and rename inside the
    /// container is one ask, forwarded by the daemon to the volume's
    /// provider and served from the volume's tree in place. No other
    /// mount may lie inside it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fuse_directory_mounts: Vec<FuseMount>,
    /// The daemon's own tools the agent holds, and how far each
    /// reaches: see [`DaemonTools`]. Absent, none. The agent's for
    /// its life.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub daemon_tools: Option<DaemonTools>,
    /// The name, unique among the caller's agents.
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
