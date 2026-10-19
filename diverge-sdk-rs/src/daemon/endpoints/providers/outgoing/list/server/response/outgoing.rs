//! One outgoing provider, as the daemon holds it.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::shared::listing::Keyed;
use crate::daemon::creator::Creator;
use crate::daemon::endpoints::providers::Status;
use crate::provider::endpoints::capacity;
use crate::daemon::endpoints::providers::outgoing::Kind;

/// One outgoing provider of the caller's: its address, which is its
/// identity, the kind of its mode without the credential, where the
/// daemon stands with it now — and, connected, what the provider
/// answered its `info` ask — when a connection last opened or closed,
/// when it was added and by whom, and its tags.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Outgoing {
    /// The address, as the add gave it: the provider's identity.
    pub address: String,
    /// Which mode it is dialled in, without the credential. See
    /// [`Kind`].
    pub kind: Kind,
    /// Where the daemon stands with it now: see [`Status`]. A
    /// connection the daemon uses carries the provider's version and
    /// capabilities as it answered them.
    pub status: Status,
    /// The provider's latest answer to the daemon's `capacity` ask, as
    /// the daemon cached it: how much the daemon may still use of it,
    /// see [`Frame`](crate::provider::endpoints::capacity::server::response::Frame).
    /// Absent until the provider has answered once, and whenever the
    /// status is not `connected`. The daemon asks again every
    /// `capacity_poll_seconds` of its configuration and after every
    /// act of its own that changes the answer; a list kept open hears
    /// each new answer as the item changed. A member the provider's
    /// capabilities exclude is not read, as the provider wire has it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capacity: Option<capacity::server::response::Frame>,
    /// When that last changed: the time the connection opened, if one
    /// is held; the time the last one closed, if none is; absent for a
    /// provider never dialled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_connected: Option<DateTime<Utc>>,
    /// When the add made it. On the wire an RFC 3339 timestamp in UTC.
    pub created: DateTime<Utc>,
    /// Who added it: the client, over an endpoint, or the agent or the
    /// tool of the client's that did so through the daemon.
    /// One [`Creator`](crate::daemon::creator::Creator), the direct
    /// maker.
    pub creator: Creator,
    /// The tags on it, as
    /// [`tag`](crate::daemon::endpoints::providers::outgoing::tag) put
    /// them. Absent when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

/// Keyed by the address: the provider's identity.
impl Keyed for Outgoing {
    type Key = String;

    fn key(&self) -> String {
        self.address.clone()
    }
}
