//! What a client's request frame carries for a message.

use bytes::Bytes;
use serde::{Deserialize, Serialize};

use crate::shared::Identity;
use crate::shared::relay::Relay;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// Send one message to `to`, through whichever of the daemon's
/// providers it listens on.
///
/// JSON, as every request of the daemon's is: the body is standard
/// base64.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Frame {
    /// The identity to message, as the providers daemons list lists it.
    pub to: Identity,
    /// The body, as standard base64 in JSON. Neither the daemon nor a
    /// provider reads it, and neither prescribes a format for it.
    #[serde(serialize_with = "crate::shared::base64::serialize", deserialize_with = "crate::shared::base64::deserialize")]
    #[schemars(with = "String")]
    pub body: Bytes,
    /// Served by another daemon: see [`Relay`]. Absent, served here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relay: Option<Relay>,
}

/// This frame's tag among the scope-opening requests. See the table in
/// [`endpoints`](crate::daemon::endpoints) for the whole allocation.
const TAG: u8 = 144;

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

/// A providers daemons message request frame that could not be read.
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
            FrameError::Empty => f.write_str("providers daemons message request frame is empty"),
            FrameError::UnexpectedTag(tag) => {
                write!(f, "expected providers daemons message request tag {TAG}, found {tag}")
            }
            FrameError::Body(error) => {
                write!(f, "providers daemons message request did not parse: {error}")
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
