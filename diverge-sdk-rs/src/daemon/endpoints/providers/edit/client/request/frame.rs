//! What a client's request frame carries for a providers edit.

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use serde::{Deserialize, Serialize};

use crate::daemon::endpoints::providers::Direction;
use crate::shared::Identity;
use crate::shared::relay::Relay;

/// Ask the daemon to change a provider's record, by its identity: every
/// member given replaces what the record holds, every member absent
/// leaves it. A direction given replaces the whole of the record's.
/// Whether it is enabled is changed only by
/// [`enable`](crate::daemon::endpoints::providers::enable) and
/// [`disable`](crate::daemon::endpoints::providers::disable).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Frame {
    /// The provider, by the identity its token establishes.
    pub identity: Identity,
    /// The direction, in place of the record's, its members beside the
    /// request's own under `direction`. Absent, the record's.
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub direction: Option<Direction>,
    /// The listable flag, in place of the record's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub listable: Option<bool>,
    /// The description, in place of the record's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Served by another daemon: see [`Relay`]. Absent, served here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relay: Option<Relay>,
}

/// This frame's tag among the scope-opening requests. See the table in
/// [`endpoints`](crate::daemon::endpoints) for the whole allocation.
const TAG: u8 = 137;

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

/// A providers edit request frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag naming some other request.
    UnexpectedTag(u8),
    /// The request did not parse.
    Body(serde_json::Error),
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameError::Empty => f.write_str("providers edit request frame is empty"),
            FrameError::UnexpectedTag(tag) => write!(f, "expected providers edit request tag {TAG}, found {tag}"),
            FrameError::Body(error) => write!(f, "providers edit request did not parse: {error}"),
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
