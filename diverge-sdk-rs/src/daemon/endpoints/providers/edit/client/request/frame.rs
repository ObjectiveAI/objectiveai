//! What a client's request frame carries for a providers edit.

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use serde::{Deserialize, Serialize};

use crate::daemon::endpoints::providers::Ip;
use crate::shared::Identity;
use crate::shared::description::Description;
use crate::shared::nullable;
use crate::shared::relay::Relay;

/// Ask the daemon to change a provider's record, by its identity. The
/// rules:
///
/// 1. **Given replaces, absent leaves.** Every member given replaces
///    what the record holds, and every member absent leaves it.
/// 2. **Three ways for two.** `announcement` and `ips` are each named
///    three ways: absent leaves the record's, `null` clears it, and a
///    value replaces it. A cleared `announcement` is a daemon that does
///    not announce itself on the provider; cleared `ips` admit any IP.
/// 3. **Not editable.** The record's identity names it and does not
///    change. Its direction — incoming or outgoing — and an outgoing
///    record's `address` do not change either: a provider reached
///    another way is a delete and an add. Whether it is enabled is
///    changed only by
///    [`enable`](crate::daemon::endpoints::providers::enable) and
///    [`disable`](crate::daemon::endpoints::providers::disable), and its
///    tags by [`tag`](crate::daemon::endpoints::providers::tag) and
///    [`untag`](crate::daemon::endpoints::providers::untag).
/// 4. **Unknown members.** A request naming any member but these —
///    `direction` and `address` among them — does not parse.
/// 5. **`ips` is an incoming record's.** `ips` given for an outgoing
///    record, as a value or as `null`, is refused with an error, and
///    nothing changes.
/// 6. **The live connection.** Nothing an edit changes closes the
///    provider's connection, except `ips` that no longer admit the live
///    connection's source address: that connection is closed then.
///    Every other change is made on the connection as it stands: the
///    daemon cancels and opens the provider's `daemons::announce`,
///    `daemons::accept` and `daemons::listen` to match the record.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Frame {
    /// The provider, by the identity its token establishes.
    pub identity: Identity,
    /// What the daemon announces itself with on the provider, in place
    /// of the record's: rule 2. `null`, the daemon stops announcing
    /// itself there.
    #[serde(default, deserialize_with = "nullable::deserialize", skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Option<Description>")]
    pub announcement: Option<Option<Description>>,
    /// The source IPs an incoming provider may dial from, in place of
    /// the record's: rules 2 and 5. `null`, any IP.
    #[serde(default, deserialize_with = "nullable::deserialize", skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Option<Vec<Ip>>")]
    pub ips: Option<Option<Vec<Ip>>>,
    /// Whether the daemon accepts connections from other daemons
    /// through the provider, in place of the record's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accept_daemons: Option<bool>,
    /// Whether the daemon listens for messages on the provider, in
    /// place of the record's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub messageable: Option<bool>,
    /// The description — the operator's note — in place of the
    /// record's.
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
