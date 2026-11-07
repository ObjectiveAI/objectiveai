//! What a client's request frame carries for a daemons add.

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use crate::shared::Identity;
use crate::daemon::endpoints::daemons::Routes;
use crate::shared::relay::Relay;

/// Ask the daemon to record a remote daemon, the account of ours it is
/// reached as, and the routes it may be reached by. The record's ID is
/// the hash of those — see
/// [`DaemonId::of`](crate::daemon::endpoints::daemons::DaemonId::of) —
/// and the answer carries it. Nothing is connected now: the first relay
/// or connected tool that names it connects.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Frame {
    /// What it is, in the caller's words. Absent, none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The remote daemon, by the identity it answers a handshake as.
    pub daemon: Identity,
    /// The identity this daemon presents to it, one of the host's: its
    /// issuer must be the remote's.
    pub identity: Identity,
    /// The routes it may be reached by: see [`Routes`]. Its members lie
    /// beside the request's own; each absent, or empty, no filter.
    #[serde(flatten)]
    pub routes: Routes,
    /// The tags to put on it. Absent, none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Served by another daemon: see [`Relay`]. Absent, served here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relay: Option<Relay>,
}

/// This frame's tag among the scope-opening requests. See the table in
/// [`endpoints`](crate::daemon::endpoints) for the whole allocation.
const TAG: u8 = 130;

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

/// A daemons add request frame that could not be read.
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
            FrameError::Empty => f.write_str("daemons add request frame is empty"),
            FrameError::UnexpectedTag(tag) => write!(f, "expected daemons add request tag {TAG}, found {tag}"),
            FrameError::Body(error) => write!(f, "daemons add request did not parse: {error}"),
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
