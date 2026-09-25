//! What a client's request frame carries for a create.

use diverge_provider_sdk::decode::Decode;
use diverge_provider_sdk::encode::{Encode, Writer};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{FuseMount, Image, Provider};

/// Ask the daemon to create an agent under a name.
///
/// Everything the agent is made from — the image, the limits, the
/// mounts, the arguments, and the provider it is pinned to, if any —
/// and the name the agent is held under from then on. What a caller may not choose is not here at all rather
/// than here and ignored: the container's name, its ports, its
/// entrypoint and its environment are the provider's, because they
/// are how the provider reaches the container and how the container
/// reaches back. There is no environment: the mounts are the
/// caller's only provisioning channel, and a field that is accepted
/// and ignored is a field callers will believe in.
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    /// The image: a name and a digest. See [`Image`].
    pub image: Image,
    /// How much memory the container may have, in BYTES.
    ///
    /// A ceiling, not a hint. A process that exceeds what the
    /// container is allowed is killed by the kernel rather than told
    /// — no failed allocation to catch, no warning first — and the
    /// container will not see this number in its own
    /// `/proc/meminfo`, which reports the host's. An image that sizes
    /// itself off what it thinks it has will size itself wrong.
    ///
    /// Bytes rather than megabytes because a unit that has to be
    /// spelled out in prose is a unit half of everyone gets wrong.
    pub memory: u64,
    /// How much the container may WRITE, in BYTES.
    ///
    /// Its own filesystem only — what it adds to or changes over the
    /// image it came from. The image's layers are read-only and are
    /// not counted, so a container starts at nothing however large
    /// the image is. It does not govern the mounts: a volume is
    /// storage that already existed, with a size of its own.
    ///
    /// Bytes rather than megabytes, for the reason
    /// [`memory`](Self::memory) gives.
    pub disk: u64,
    /// The one provider the agent runs on, and the volumes of that
    /// provider made visible inside the container. See [`Provider`].
    ///
    /// Absent, the agent runs on whichever provider the daemon
    /// chooses, and mounts no volume: a volume is a provider's own
    /// and does not carry across, so an agent with state on a
    /// provider's disk is an agent of that provider.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<Provider>,
    /// Files of the daemon's host served LIVE, mounted one each over
    /// FUSE.
    ///
    /// Each names a file by its path on the host and its path in the
    /// container — see [`FuseMount`]. Every one is mounted before the
    /// agent runs, and every open and every changed close inside the
    /// container is one ask to the daemon, served from the host's
    /// file. The file is overwritten in place only; a program that
    /// replaces its file by rename needs a directory mount. Its
    /// container path is no other mount's and lies inside none, as
    /// every mount's.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fuse_file_mounts: Vec<FuseMount>,
    /// Directories of the daemon's host served LIVE, mounted one each
    /// over FUSE.
    ///
    /// Each names a directory by its path on the host and its path in
    /// the container — see [`FuseMount`]. The whole tree under the
    /// host path is what the container sees: every listing, read,
    /// write, creation, removal and rename inside the container is one
    /// ask to the daemon, served from the host's tree. No other mount
    /// may lie inside it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fuse_directory_mounts: Vec<FuseMount>,
    /// What the image is told once, as the image defines it, for the
    /// agent's life.
    ///
    /// A JSON value, because this crate does not know what an image
    /// takes — a model, tools, an image's own knobs — and a wire that
    /// typed it would have to be revised for every image that ever
    /// ran. It is handed to the container and not read here.
    pub arguments: Value,
    /// The name, unique among the caller's agents.
    pub name: String,
}

/// This frame's tag among the scope-opening requests.
///
/// One byte at the front of the payload, which is what tells a reader
/// which request it holds. The frame layer does not discriminate them
/// — [`ClientFrame::Request`](diverge_provider_sdk::frame::client::ClientFrame::Request)
/// is one type carrying bytes — so the distinction has to be in the
/// bytes, and each request owns the value that names it.
///
/// See the table in [`endpoints`](crate::endpoints) for the whole
/// allocation. The values are chosen across modules that do not know
/// about each other, so the table is the only place they can be seen
/// at once.
const TAG: u8 = 0;

/// JSON, as the container request is on the provider's wire: the
/// arguments are a JSON value, and a value cannot come back out of
/// postcard at all.
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
