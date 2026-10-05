//! One outgoing provider, as the daemon holds it.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::daemon::creator::Creator;
use crate::daemon::endpoints::providers::outgoing::Kind;

/// One outgoing provider of the caller's: its address, which is its
/// identity, the kind of its mode without the credential, whether the
/// daemon holds a connection to it now, when it last did, when it was
/// added and by whom.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Outgoing {
    /// The address, as the add gave it: the provider's identity.
    pub address: String,
    /// Which mode it is dialled in, without the credential. See
    /// [`Kind`].
    pub kind: Kind,
    /// Whether the daemon holds a connection to it now.
    pub connected: bool,
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
}
