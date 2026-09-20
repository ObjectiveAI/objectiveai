//! What a client's request frame carries for a tag.

use diverge_provider_sdk::decode::Decode;
use diverge_provider_sdk::encode::{Encode, Writer};
use diverge_provider_sdk::shared::containers::request::Container;
use serde::{Deserialize, Serialize};

/// Ask the daemon to spawn an agent under a tag.
///
/// Everything the agent container is made from — the
/// [`Container`] of a provider's `containers::agents::run`, verbatim,
/// so the daemon relays it to whichever provider it chooses and reads
/// nothing of it but what it needs to choose — and the tag the agent
/// is held under from then on. The wire is one object: the
/// container's members, flattened, and `tag`.
///
/// # The tag
///
/// A string of the caller's choosing, unique among the caller's
/// agents: the daemon refuses a request whose tag names an agent that
/// is running, and says so with a variant of its own, because a
/// caller acts on it differently from a failure — use the agent it
/// has, or choose another tag. Nothing here constrains the string's
/// form; the tag is the caller's word for its agent, and the daemon
/// compares it and does not read it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    /// The agent container, as a provider would be asked for it.
    #[serde(flatten)]
    pub container: Container,
    /// The tag, unique among the caller's running agents.
    pub tag: String,
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

/// A tag request frame that could not be read.
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
            FrameError::Empty => f.write_str("agents tag request frame is empty"),
            FrameError::UnexpectedTag(tag) => {
                write!(f, "expected agents tag request tag {TAG}, found {tag}")
            }
            FrameError::Body(error) => {
                write!(f, "agents tag request did not parse: {error}")
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
