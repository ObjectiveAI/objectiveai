//! What a client's request frame carries for a list.

use chrono::{DateTime, Utc};
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use serde::{Deserialize, Serialize};

use super::Kind;

/// Ask the daemon for the caller's tools, narrowed.
///
/// Everything is optional, and a request with none of it — `{}` on the
/// wire — is every tool of the caller's. The members but `jq` and
/// `count` together are the filter. The daemon applies the filter
/// first, oldest created first, so the program sees only what it lets
/// through, and runs the program over each tool of that; what the
/// program yields is what comes back, and without a program the tools
/// come back as they are. The count caps what comes back.
/// [`Tool`](crate::daemon::endpoints::tools::list::server::response::Tool)
/// is the shape each comes back in without a program, and the reference
/// for what a program is run over.
///
/// # Any one of, every one of
///
/// A member that lists candidates — `names`, `templates` — matches a
/// tool that is any one of them. `agents` matches a tool attached to
/// every one of them, `all_tags` one that carries every one of them,
/// and `any_tags` one that carries any one of them. An empty list is
/// absent, and matches every tool.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Frame {
    /// Any one of these names, as a create or a connect gave them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub names: Vec<String>,
    /// A created tool made from any one of these templates, by id; a
    /// connected tool matches none of them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub templates: Vec<String>,
    /// Created by the daemon, or connected to somebody else's: the
    /// `kind` an
    /// [`Origin`](crate::daemon::endpoints::tools::list::server::response::Origin)
    /// is tagged with, see [`Kind`]; absent, either.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<Kind>,
    /// Whether active — its container running, or its connect scope
    /// held — `true`, or not, `false`; absent, either.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
    /// Attached to every one of these agents, by name. Absent when
    /// empty, and then any attachments.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub agents: Vec<String>,
    /// Every one of these among the tool's tags, as
    /// [`tag`](crate::daemon::endpoints::tools::tag) put them. Absent
    /// when empty, and then any tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub all_tags: Vec<String>,
    /// Any one of these among the tool's tags, as
    /// [`tag`](crate::daemon::endpoints::tools::tag) put them; with
    /// `all_tags`, both hold. Absent when empty, and then any tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub any_tags: Vec<String>,
    /// The earliest `created` to list, inclusive; absent, no earliest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_from: Option<DateTime<Utc>>,
    /// The latest `created` to list, inclusive; absent, no latest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_to: Option<DateTime<Utc>>,
    /// A jq program, as the `jq` command takes one, run with each
    /// matching tool — one
    /// [`Tool`](crate::daemon::endpoints::tools::list::server::response::Tool)
    /// as JSON — as its input; everything it yields comes back, in
    /// order. So `.name` is every matching tool's name, as a string.
    /// Absent, the tools come back as they are. The daemon does not
    /// read the program beyond running it; one that will not compile,
    /// or fails while it runs, is the scope's error.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jq: Option<String>,
    /// How many values to send at most, counting what comes back —
    /// tools as they are, or what the program yields — and not what the
    /// filter reads; once that many have been sent the scope finishes,
    /// whether or not more would have matched. `0` sends nothing and
    /// finishes at once. Absent, no cap.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<u64>,
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
const TAG: u8 = 19;

/// JSON, as every request of the daemon's is: the filter's members,
/// each absent when it says nothing, so that a request that says
/// nothing is `{}`.
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

/// A tools list request frame that could not be read.
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
            FrameError::Empty => f.write_str("tools list request frame is empty"),
            FrameError::UnexpectedTag(tag) => {
                write!(f, "expected tools list request tag {TAG}, found {tag}")
            }
            FrameError::Body(error) => {
                write!(f, "tools list request did not parse: {error}")
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
