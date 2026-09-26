//! What a client's request frame carries for a logs read.

use chrono::{DateTime, Utc};
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use serde::{Deserialize, Serialize};

use super::ItemType;

/// Ask the daemon for an agent's log, narrowed, and perhaps
/// watched.
///
/// The name is the one a [`create`](crate::daemon::endpoints::agents::create)
/// gave the agent. Everything else is optional. The spans, the type
/// and the program together are the filter, and a request with none
/// of them is the whole log; the count caps what the filter yields.
/// The spans are inclusive at both ends.
/// The daemon applies the spans and the type before the program, so
/// the program sees only what they let through, and runs over each
/// [`ItemWrapper`] of that, oldest first; what the program yields is
/// what comes back, and without one the items come back as they
/// are.
///
/// # Historical, then live
///
/// The daemon runs the filter over the log as it stands at the
/// request, oldest first, and sends every value. Without `watch`
/// that is the read, and the scope finishes. With `watch` the daemon
/// then runs the same filter over each item as it lands, sends what
/// it yields as it yields it, and keeps the scope open. The daemon
/// does not mark where the historical read ended and the live one
/// began; an item kept as the one gives way to the other is sent
/// once, not twice and not never — that is the daemon's to get
/// right, and the index says whether it did.
///
/// # When a watch ends
///
/// A watch ends on its own only when nothing more can come back:
/// the count is met, or the filter can never match again, which the
/// daemon decides from the spans alone — the type and the program
/// never rule an item out before it is seen — and from the fact
/// that the log's indexes and times only go up: a later item never
/// has a smaller `logs_index` or an earlier `created`. So:
///
/// - `count` given: the watch ends once that many values have been
///   sent, and the scope finishes after the last of them.
/// - `logs_index_to` given: the watch ends once the log holds an
///   item whose `logs_index` is `logs_index_to` or greater. That
///   item is sent first if it matches, and the scope finishes after
///   it.
/// - `created_to` given: the watch ends once the log holds an item
///   whose `created` is later than `created_to`, or once the
///   daemon's clock passes `created_to` with no such item — both
///   mean nothing later can be in the span. An item at the bound is
///   in the span and is sent first if it matches.
/// - Several given: whichever comes first.
/// - None given: the watch never ends on its own. It ends when
///   the client [cancels](crate::daemon::endpoints::agents::logs::client::channel_request::Frame::Cancel),
///   closes the scope, or the agent is deleted.
///
/// Every watch also ends on a cancel, on the agent's deletion, and
/// on an error. `logs_index_from` and `created_from` never end one:
/// an item before them is skipped, and one after can always come.
///
/// [`ItemWrapper`]: crate::daemon::endpoints::agents::logs::server::response::ItemWrapper
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    /// The agent's name, as its create gave it.
    pub name: String,
    /// The first `logs_index` to read, inclusive; absent, the log's
    /// first.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logs_index_from: Option<u64>,
    /// The last `logs_index` to read, inclusive; absent, the log's
    /// last — and, watching, no last at all.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logs_index_to: Option<u64>,
    /// The earliest `created` to read, inclusive; absent, the log's
    /// first.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_from: Option<DateTime<Utc>>,
    /// The latest `created` to read, inclusive; absent, the log's
    /// last — and, watching, no last at all.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_to: Option<DateTime<Utc>>,
    /// One kind of item, by its `type` as an [`ItemWrapper`](crate::daemon::endpoints::agents::logs::server::response::ItemWrapper) carries
    /// it; absent, every kind.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ItemType>,
    /// A jq program, as the `jq` command takes one, run with each
    /// matching item as its input; everything it yields comes back,
    /// in order. So `.text` on `type: user_text_content` is every
    /// text a caller sent, as strings. Absent, the items come back as
    /// they are. The daemon does not read the program beyond running
    /// it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jq: Option<String>,
    /// How many values to send at most, counting what comes back —
    /// items as they are, or what the program yields — and not what
    /// the filter reads; once that many have been sent the scope
    /// finishes, whether or not more would have matched. `0` sends
    /// nothing and finishes at once. Absent, no cap.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<u64>,
    /// Whether to watch: `true`, the daemon sends what matches now
    /// and then runs the filter over each item as it lands, until
    /// the count is met, the filter can never match again, a cancel,
    /// or the agent's deletion; `false` or absent, the daemon sends
    /// what matches now and finishes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub watch: Option<bool>,
}

/// This frame's tag among the scope-opening requests.
///
/// One byte at the front of the payload, which is what tells a reader
/// which request it holds. The frame layer does not discriminate them
/// — [`ClientFrame::Request`](crate::wire::frame::client::ClientFrame::Request)
/// is one type carrying bytes — so the distinction has to be in the
/// bytes, and each request owns the value that names it.
///
/// See the table in [`endpoints`](crate::daemon::endpoints) for the whole
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
