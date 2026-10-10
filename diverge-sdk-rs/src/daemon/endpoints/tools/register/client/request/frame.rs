//! What a client's request frame carries for a register.

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use serde::{Deserialize, Serialize};

use crate::daemon::reference;

/// Ask the daemon to hold, under a name, a tool another daemon holds.
///
/// Two things name the tool — the daemon it is on, by a record of
/// [`providers::daemons`](crate::daemon::endpoints::providers::daemons),
/// and the tool as that daemon names it — and the name it is held
/// under here from then on. Nothing here is what the container is made
/// from: its image, its limits, its mounts and its arguments are the
/// other daemon's, stated in its own
/// [`create`](crate::daemon::endpoints::tools::create), and a connected
/// tool is not [`edit`](crate::daemon::endpoints::tools::edit)ed.
///
/// # Nothing is connected now
///
/// The daemon records the tool and answers. While an agent the tool is
/// attached to uses it, the daemon connects to the daemon named —
/// through a provider both are connected to, as the record's links
/// say — and opens that daemon's
/// [`connect`](crate::daemon::endpoints::tools::connect) naming the
/// tool, over which every MCP exchange of the agent's travels; it lets
/// the connection go when nothing has used the tool for
/// `idle_seconds`. It never starts or stops the container itself: the
/// connect does, there. A daemon that cannot be reached, or a tool it
/// does not have or does not allow, is not this request's error: it is
/// the tool inactive in a
/// [`list`](crate::daemon::endpoints::tools::list), and the agent's
/// tool calls failing.
///
/// # The name
///
/// Optional. A string of the caller's choosing, unique among the
/// caller's tools, created and connected alike: the daemon refuses a
/// request whose name is a tool's already, and says so with a variant
/// of its own, because a caller acts on it differently from a failure —
/// use the tool it has, or choose another name. A request with no name
/// is never refused for one: the tool is reached by the daemon and
/// tool it is registered from, which it always has. Nothing here
/// constrains the string's form; the daemon compares it and does not
/// read it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Frame {
    /// The daemon the tool is on, by the name of its record: see
    /// [`providers::daemons`](crate::daemon::endpoints::providers::daemons).
    /// A name no record of the caller's has is the register's error.
    pub daemon: String,
    /// The tool, as that daemon names it: by its name there, or by its
    /// template and its index there. See [`reference::Tool`]. Compared
    /// by this daemon, and read by that one.
    pub tool: reference::Tool,
    /// The name, if any: a string of the caller's choosing, unique
    /// among the caller's tools, by which the tool is reached
    /// afterwards beside the daemon and tool it is registered from.
    /// Absent, the tool has none, and is reached by that pair only.
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
const TAG: u8 = 18;

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

/// A tools register request frame that could not be read.
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
            FrameError::Empty => f.write_str("tools register request frame is empty"),
            FrameError::UnexpectedTag(tag) => {
                write!(f, "expected tools register request tag {TAG}, found {tag}")
            }
            FrameError::Body(error) => {
                write!(f, "tools register request did not parse: {error}")
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
