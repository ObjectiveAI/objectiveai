//! What a client's request frame carries for an edit.

use serde::{Deserialize, Serialize};

use crate::daemon::reference;

use crate::daemon::endpoints::agents::create::client::request::{FuseMount, VolumeMount};
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// Ask the daemon to change what a tool mounts, by name.
///
/// The name is the one a
/// [`create`](crate::daemon::endpoints::tools::create) gave the
/// tool, compared as the daemon compares names and not read. The
/// three lists are absolute: what the tool mounts after the edit is
/// exactly what they state, each list replacing the tool's list of
/// that kind whole, an empty list leaving the tool with no mount of
/// that kind. A mount the tool already has is kept where the new
/// list names it the same; a mount the new list leaves out is gone;
/// a mount the new list adds is made. A tool that is active — its
/// container running, an agent it is attached to being active — is
/// not edited, and the daemon says so with a variant of its own,
/// because a caller acts on it differently from a failure: wait for
/// the container to stop, and ask again. The tool's image, limits,
/// provider and arguments are not edited: they are its for its life.
/// A [`connect`](crate::daemon::endpoints::tools::connect)ed tool has
/// no mounts of this caller's and is not edited; the daemon says so
/// with a variant of its own.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Frame {
    /// The tool: by its name, as its create or its connect gave it,
    /// or by its template and its index, which name a created tool
    /// once and for all. See [`reference::Tool`].
    pub tool: reference::Tool,
    /// Volumes of the provider the tool is pinned to, as the
    /// create's [`Provider`](crate::daemon::endpoints::agents::create::client::request::Provider)
    /// names them: see [`VolumeMount`]. A tool pinned to no provider
    /// mounts no volume, and a request that names one for such a
    /// tool is the edit's error. Ordered, and applied in
    /// order; no mount's path, in any list, is a prefix of another's.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub volume_mounts: Vec<VolumeMount>,
    /// Files of providers' volumes served live across the daemon,
    /// mounted one each over FUSE: see [`FuseMount`], and the
    /// create's
    /// [`fuse_file_mounts`](crate::daemon::endpoints::agents::create::client::request::Frame::fuse_file_mounts).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fuse_file_mounts: Vec<FuseMount>,
    /// Directories of providers' volumes served live across the
    /// daemon, mounted one each over FUSE: see [`FuseMount`], and the
    /// create's
    /// [`fuse_directory_mounts`](crate::daemon::endpoints::agents::create::client::request::Frame::fuse_directory_mounts).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fuse_directory_mounts: Vec<FuseMount>,
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
const TAG: u8 = 17;

/// JSON, as the create is: the same mount types, and the same reader
/// for every request of the daemon's.
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

/// A tools edit request frame that could not be read.
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
            FrameError::Empty => f.write_str("tools edit request frame is empty"),
            FrameError::UnexpectedTag(tag) => {
                write!(f, "expected tools edit request tag {TAG}, found {tag}")
            }
            FrameError::Body(error) => {
                write!(f, "tools edit request did not parse: {error}")
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
