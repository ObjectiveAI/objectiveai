//! What a client's request frame carries for a providers add.

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use serde::{Deserialize, Serialize};

use crate::daemon::endpoints::providers::Direction;
use crate::shared::Identity;
use crate::shared::description::Description;
use crate::shared::relay::Relay;

/// Ask the daemon to keep a provider's record, and to work with it from
/// now on: dial it when it is outgoing, admit it when it is incoming.
/// What the daemon holds open on the provider while it is connected is
/// three members' word, each independent of the other two:
///
/// 1. **`announcement`.** Present, the daemon announces itself on the
///    provider with that text — it holds the provider's
///    `daemons::announce` open, and is in its `daemons::list`. Absent,
///    it does not announce itself there.
/// 2. **`accept_daemons`.** `true`, the daemon accepts connections from
///    other daemons through the provider — it holds the provider's
///    `daemons::accept` open. Absent, `true`.
/// 3. **`messageable`.** `true`, the daemon listens for messages on the
///    provider — it holds the provider's `daemons::listen` open.
///    Absent, `false`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Frame {
    /// The provider, by the identity its token establishes.
    pub identity: Identity,
    /// How the connection is made: see [`Direction`]. Its members lie
    /// beside the request's own, under `direction`.
    #[serde(flatten)]
    pub direction: Direction,
    /// What the daemon announces itself with on the provider: rule 1.
    /// Absent, the daemon does not announce itself there.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub announcement: Option<Description>,
    /// Whether the daemon accepts connections from other daemons
    /// through the provider: rule 2. Absent, `true`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accept_daemons: Option<bool>,
    /// Whether the daemon listens for messages on the provider: rule 3.
    /// Absent, `false`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub messageable: bool,
    /// Whether the daemon works with it from now on. Absent, `true`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// What it is, in the caller's words: the operator's note, which
    /// the provider never sees. Absent, none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The tags to put on it. Absent, none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Served by another daemon: see [`Relay`]. Absent, served here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relay: Option<Relay>,
}

/// This frame's tag among the scope-opening requests. See the table in
/// [`endpoints`](crate::daemon::endpoints) for the whole allocation.
const TAG: u8 = 136;

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

/// A providers add request frame that could not be read.
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
            FrameError::Empty => f.write_str("providers add request frame is empty"),
            FrameError::UnexpectedTag(tag) => write!(f, "expected providers add request tag {TAG}, found {tag}"),
            FrameError::Body(error) => write!(f, "providers add request did not parse: {error}"),
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
