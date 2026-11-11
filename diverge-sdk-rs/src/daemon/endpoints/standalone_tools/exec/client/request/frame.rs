//! The request that runs a command.

use serde::{Deserialize, Serialize};

use crate::daemon::reference;
use crate::shared::containers::exec;
use crate::shared::relay::Relay;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// Run one command in a standalone tool's container. One JSON object: the
/// tool, then the command's own members — `argv`, `cwd`, `env` — as
/// [`exec::request::Request`] states them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Frame {
    /// Whose container.
    pub tool: reference::Tool,
    /// The command.
    #[serde(flatten)]
    pub command: exec::request::Request,
    /// Served by another daemon: see [`Relay`]. Absent, served here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relay: Option<Relay>,
}

/// This frame's tag among the scope-opening requests.
const TAG: u8 = 116;

impl Encode for Frame {
    /// The ordinary JSON failure.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        out.extend_from_slice(&[TAG]);
        serde_json::to_writer(out, self)
    }
}

impl Decode<'_> for Frame {
    /// See [`FrameError`].
    type Error = FrameError;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        if *tag != TAG {
            return Err(FrameError::UnexpectedTag(*tag));
        }
        serde_json::from_slice(rest).map_err(FrameError::Body)
    }
}

/// A standalone tools exec request that did not decode.
#[derive(Debug)]
pub enum FrameError {
    /// No tag at all.
    Empty,
    /// Another request's tag.
    UnexpectedTag(u8),
    /// The JSON did not parse.
    Body(serde_json::Error),
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameError::Empty => f.write_str("standalone tools exec request frame is empty"),
            FrameError::UnexpectedTag(tag) => write!(f, "expected standalone tools exec request tag {TAG}, found {tag}"),
            FrameError::Body(error) => write!(f, "standalone tools exec request did not parse: {error}"),
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
