//! One outgoing provider, as the daemon holds it.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::daemon::creator::Creator;
use crate::daemon::endpoints::providers::Status;
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
