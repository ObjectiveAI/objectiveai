//! What a client's request frame carries for an edit.

use serde::{Deserialize, Serialize};

use crate::daemon::reference;

use crate::daemon::edit::Edit;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// Ask the daemon to change a tool: its name, its account, its
/// mounts, its deployer.
///
/// The tool is named by its name, by its template and its index, or
/// by the provider and id it joined, as
/// [`reference`](crate::daemon::reference) states. Every other member
/// is an optional `delete` or `set`, replacing the tool's whole: see
/// [`Edit`]. A tool that is active — its container running, or its
/// connect scope held — has its mounts left as they are, and the
/// daemon says so with a variant of its own; a request that names no
/// mount is applied live. A
/// [`connect`](crate::daemon::endpoints::tools::connect)ed tool has
/// no mounts of this caller's and runs under no account, since it
/// runs nothing of the caller's: a request naming either for one is
/// refused as not owned, and only its name changes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Frame {
    /// The tool: by its name, as its create or its connect gave it,
    /// or by its template and its index, which name a created tool
    /// once and for all. See [`reference::Tool`].
    pub tool: reference::Tool,
    /// What an agent's edit and a tool's share — the name, the account,
    /// the mounts, the deployer, every one optional: see
    /// [`Edit`]. Flattened, so its members are this object's own.
    #[serde(flatten)]
    pub edit: Edit,
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
