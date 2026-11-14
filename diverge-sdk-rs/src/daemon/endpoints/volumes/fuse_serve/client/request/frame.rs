//! What a client's request frame carries for a volume FUSE serve.

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use serde::{Deserialize, Serialize};
use crate::daemon::reference;
use crate::provider::endpoints::volumes::Mode;
use crate::shared::relay::Relay;

/// Ask the daemon to serve a volume over FUSE: the provider's
/// [`volumes::fuse_serve`](crate::provider::endpoints::volumes::fuse_serve)
/// request, the volume named as the daemon names one. See
/// [`fuse_serve`](super::super::super) for what answers it and what
/// the scope then carries.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Frame {
    /// The volume: see [`reference::Volume`].
    pub volume: reference::Volume,
    /// The mode the volume is meant to be in. A volume in another mode
    /// is refused, with the mode it is in.
    pub mode: Mode,
    /// The most bytes the serve's own layer may hold, for an ephemeral
    /// volume, whose changes land in that layer and go with the serve.
    /// Absent, none: what a persistent or a read-only volume states.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overlay_disk: Option<u64>,
    /// Served by another daemon: see [`Relay`]. Absent, served here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relay: Option<Relay>,
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
const TAG: u8 = 158;

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

/// A volumes fuse serve request frame that could not be read.
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
            FrameError::Empty => f.write_str("volumes fuse serve request frame is empty"),
            FrameError::UnexpectedTag(tag) => {
                write!(f, "expected volumes fuse serve request tag {TAG}, found {tag}")
            }
            FrameError::Body(error) => {
                write!(f, "volumes fuse serve request did not parse: {error}")
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
