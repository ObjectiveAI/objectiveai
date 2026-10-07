//! What a client's request frame carries for a connect.

use serde::{Deserialize, Serialize};

use crate::daemon::endpoints::agents::logs::server::response::Identity;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// Ask the daemon to hold, under a name, a tool container somebody else
/// runs.
///
/// The three things a runner hands to whoever it lets in — the provider
/// its container runs on, the container's id, and an authorization —
/// and the name the tool is held under from then on. Nothing here is
/// what the container is made from: its image, its limits, its mounts
/// and its arguments are its runner's, stated in the runner's own
/// [`create`](crate::daemon::endpoints::tools::create), and a connected
/// tool is not [`edit`](crate::daemon::endpoints::tools::edit)ed. Whom
/// a runner's daemon admits is an
/// [admission](crate::daemon::endpoints::tools::admit) on the tool
/// there, and the authorization is the key that admission answered.
///
/// # Nothing is joined now
///
/// The daemon records the tool and answers. It opens the provider
/// protocol's `containers::tools::connect` on the provider named, with
/// the id and the authorization, while an agent the tool is attached to
/// is active, and lets the scope go when none is; it never starts or
/// stops the container. A container that is not there, or an
/// authorization the runner declines, is not this request's error: it
/// is the tool inactive in a
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
/// is never refused for one: the tool is reached by the provider and id
/// it joined, which it always has. Nothing here constrains the string's
/// form; the daemon compares it and does not read it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Frame {
    /// The provider the container runs on, as the daemon knows it: the
    /// same object an agent's log names a provider by, and a
    /// [`FuseMount`](crate::daemon::endpoints::agents::create::client::request::FuseMount)
    /// names one by — `kind: "outgoing"` and the address the daemon
    /// dials, or `kind: "incoming_unbrokered"` and the identity the
    /// daemon's judging of the provider's credential answered. A
    /// provider the daemon does not know by that identity is the
    /// connect's error.
    pub provider: Identity,
    /// The container's id, as its runner's run answered it: what the
    /// provider protocol's connect names a container by.
    pub id: String,
    /// The authorization the runner judges: the string the provider
    /// relays to the runner as an authorize ask, whose answer is
    /// whether the connection opens. Its form is the runner's to state.
    pub authorization: String,
    /// The name, if any: a string of the caller's choosing, unique
    /// among the caller's tools, by which the tool is reached
    /// afterwards beside the provider and id it joined. Absent, the
    /// tool has none, and is reached by that pair only.
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

/// JSON, as the rest of the daemon's requests are.
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

/// A tools connect request frame that could not be read.
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
            FrameError::Empty => f.write_str("tools connect request frame is empty"),
            FrameError::UnexpectedTag(tag) => {
                write!(f, "expected tools connect request tag {TAG}, found {tag}")
            }
            FrameError::Body(error) => {
                write!(f, "tools connect request did not parse: {error}")
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
