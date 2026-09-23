//! What a client's request frame carries for a logs read.

use chrono::{DateTime, Utc};
use diverge_provider_sdk::decode::Decode;
use diverge_provider_sdk::encode::{Encode, Writer};
use serde::{Deserialize, Serialize};

/// Ask the daemon for an agent's log, narrowed, and perhaps kept
/// open.
///
/// The name is the one a [`create`](crate::endpoints::agents::create)
/// gave the agent. Everything else is optional, and each narrows what
/// comes back; a request with none of them is the whole log. The
/// spans are inclusive at both ends. The daemon applies the spans
/// and the type before the program, so the program sees only what
/// they let through, and runs over each [`Item`] of that, oldest
/// first; what the program yields is what comes back, and without
/// one the items come back as they are.
///
/// [`Item`]: crate::endpoints::agents::logs::server::response::Item
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    /// The agent's name, as its create gave it.
    pub name: String,
    /// The first `logs_id` to read, inclusive; absent, the log's
    /// first.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logs_id_from: Option<u64>,
    /// The last `logs_id` to read, inclusive; absent, the log's last
    /// — and, subscribed, no last at all.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logs_id_to: Option<u64>,
    /// The earliest `created` to read, inclusive; absent, the log's
    /// first.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_from: Option<DateTime<Utc>>,
    /// The latest `created` to read, inclusive; absent, the log's
    /// last — and, subscribed, no last at all.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_to: Option<DateTime<Utc>>,
    /// One kind of item, by its `type` as an [`Item`] carries it —
    /// `user_text_content`, `assistant_tool_call`, `usage`, and the
    /// rest — or `error` for the errors; absent, every kind.
    ///
    /// [`Item`]: crate::endpoints::agents::logs::server::response::Item
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    /// A jq program, as the `jq` command takes one, run with each
    /// matching item as its input; everything it yields comes back,
    /// in order. So `.text` on `type: user_text_content` is every
    /// text a caller sent, as strings. Absent, the items come back as
    /// they are. The daemon does not read the program beyond running
    /// it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jq: Option<String>,
    /// Whether to stay open: `true`, the daemon sends what matches
    /// and then each item that lands after, as it lands, for as long
    /// as the client keeps the scope, and says when the agent goes
    /// idle; `false` or absent, the daemon sends what matches and
    /// finishes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subscribe: Option<bool>,
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
const TAG: u8 = 3;

/// JSON, as the rest of the agents family's requests are.
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

/// A logs request frame that could not be read.
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
            FrameError::Empty => f.write_str("agents logs request frame is empty"),
            FrameError::UnexpectedTag(tag) => {
                write!(f, "expected agents logs request tag {TAG}, found {tag}")
            }
            FrameError::Body(error) => {
                write!(f, "agents logs request did not parse: {error}")
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
