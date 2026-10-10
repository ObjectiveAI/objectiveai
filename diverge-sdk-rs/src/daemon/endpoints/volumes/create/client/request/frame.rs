//! What a client's request frame carries for a create.

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use serde::{Deserialize, Serialize};

use crate::daemon::endpoints::agents::logs::server::response::Identity;
use crate::provider::endpoints::volumes::Mode;
/// Ask the daemon to create a volume on a provider, as the provider
/// protocol's
/// [`volumes::create`](crate::provider::endpoints::volumes::create)
/// takes one: a name of the caller's choosing, how many bytes it
/// reserves, and its mode. A name a volume of that provider's has
/// already is refused.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Frame {
    /// The provider to make it on: see [`Identity`].
    pub provider: Identity,
    /// What to call it: the name the provider lists it under from then
    /// on, and the whole of how it is named. What a provider accepts as
    /// a name is the provider's to state.
    pub name: String,
    /// How many bytes it reserves, to start with; an
    /// [`edit`](crate::daemon::endpoints::volumes::edit) changes it
    /// later.
    pub bytes: u64,
    /// The mode it starts in: `persistent`, `ephemeral` or `read_only`,
    /// as the provider's [`Mode`] states each.
    pub mode: Mode,
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
const TAG: u8 = 65;

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

/// A volumes create request frame that could not be read.
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
            FrameError::Empty => f.write_str("volumes create request frame is empty"),
            FrameError::UnexpectedTag(tag) => {
                write!(f, "expected volumes create request tag {TAG}, found {tag}")
            }
            FrameError::Body(error) => {
                write!(f, "volumes create request did not parse: {error}")
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
