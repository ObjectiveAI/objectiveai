//! What a client's request frame carries for a list.

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use serde::{Deserialize, Serialize};

use crate::shared::relay::Relay;

use super::Filter;
/// Ask the daemon for the events, narrowed.
///
/// Everything is optional, and a request with none of it — `{}` on the
/// wire — is every event, when the account holds `list` over events.
/// The members but `count` together are the filter. The daemon
/// applies the filter and sends each event it lets through as an
/// [`Event`](crate::daemon::endpoints::events::Event), in id order,
/// and keeps sending as events are added and their counts raised. The
/// count keeps the initial listing to the latest that many; what
/// follows it has no cap.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Frame {
    /// The filter: see [`Filter`]. Flattened, so its members are this
    /// object's own.
    #[serde(flatten)]
    pub filter: Filter,
    /// How many events the initial listing is kept to, counting what
    /// matches and not what the filter reads: the LATEST that many by
    /// id are the listing. What comes after the word that the list is
    /// whole is every event added or changed, with no cap — the window
    /// bounds what was there when the scope opened, not what comes in;
    /// an event the window left out whose count is raised after
    /// arrives as added, since the client has not seen it. `0` is a
    /// listing of nothing, told whole at once, and watched. Absent, no
    /// cap: every event that matches.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<u64>,
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
const TAG: u8 = 109;

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

/// An events list request frame that could not be read.
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
            FrameError::Empty => f.write_str("events list request frame is empty"),
            FrameError::UnexpectedTag(tag) => {
                write!(f, "expected events list request tag {TAG}, found {tag}")
            }
            FrameError::Body(error) => {
                write!(f, "events list request did not parse: {error}")
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
